//! AI, behind three locks: a key, the master switch and the feature's own
//! switch. This holds the keys (in the OS credential store, never in a file
//! of ours and never handed back to the window) and makes the request, to
//! whichever service was chosen: Claude, `OpenAI`, or any other that speaks
//! the same chat protocol.
//!
//! The app asks nothing on its own. Every request here begins with a click
//! on something that says what will be sent, and to whom.

use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::ai::{self, AiError, Ask};
use crate::core::settings::{AiSettings, Provider};

/// Where the keys are kept in the OS credential store.
const SERVICE: &str = "dev.bajpai.keepmeawake";

/// How long one request may take. The answers are a few sentences from a
/// small model, so this is for a slow network, not a slow answer.
const TIMEOUT: Duration = Duration::from_secs(45);

/// What the window may know about the key: whether there is one, for the
/// service chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiStatus {
    /// A key is stored for the chosen service.
    pub has_key: bool,
}

/// The name a service's key is kept under. Each service has its own, so
/// changing service never sends one service's key to another.
pub fn account(provider: Provider) -> &'static str {
    match provider {
        Provider::Anthropic => "anthropic-api-key",
        Provider::OpenAi => "openai-api-key",
        Provider::Custom => "custom-api-key",
    }
}

/// Somewhere secrets can be kept, by name.
pub trait KeyStore: Send + Sync {
    /// The key kept under `account`, if there is one.
    fn get(&self, account: &str) -> Option<String>;
    /// Stores `key` under `account`, or removes it with `None`.
    fn set(&self, account: &str, key: Option<&str>) -> Result<(), String>;
}

/// The OS credential store: Credential Manager on Windows, the Keychain on
/// a Mac.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsKeyStore;

impl OsKeyStore {
    fn entry(account: &str) -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, account).map_err(|err| err.to_string())
    }
}

impl KeyStore for OsKeyStore {
    fn get(&self, account: &str) -> Option<String> {
        let key = Self::entry(account).ok()?.get_password().ok()?;
        Some(key).filter(|key| !key.trim().is_empty())
    }

    fn set(&self, account: &str, key: Option<&str>) -> Result<(), String> {
        let entry = Self::entry(account)?;
        match key {
            Some(key) => entry.set_password(key).map_err(|err| err.to_string()),
            None => match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(err) => Err(err.to_string()),
            },
        }
    }
}

/// Which feature is being asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// The Almanac's week, in words.
    WeekInWords,
    /// Home's sentence, turned into a plan.
    TellTheSun,
    /// The Almanac's day, in a line.
    DayInALine,
    /// A question about the days in view.
    AskTheAlmanac,
}

/// The keys and the client that asks.
pub struct Assistant {
    keys: Box<dyn KeyStore>,
    /// Built on first use: most runs of the app never ask.
    client: Mutex<Option<reqwest::Client>>,
}

impl Assistant {
    /// An assistant with its keys in `keys`.
    pub fn new(keys: Box<dyn KeyStore>) -> Self {
        Self {
            keys,
            client: Mutex::new(None),
        }
    }

    /// Whether a key is stored for `provider`.
    pub fn status(&self, provider: Provider) -> AiStatus {
        AiStatus {
            has_key: self.keys.get(account(provider)).is_some(),
        }
    }

    /// Stores `provider`'s key, or removes it with `None` or nothing but
    /// spaces.
    pub fn set_key(&self, provider: Provider, key: Option<&str>) -> Result<AiStatus, String> {
        let key = key.map(str::trim).filter(|key| !key.is_empty());
        self.keys.set(account(provider), key)?;
        Ok(self.status(provider))
    }

    /// The chosen service's key, if all three locks are open for `feature`:
    /// a key is stored for that service, AI is switched on, and so is the
    /// feature. Hidden, AI is off whatever else is set: what cannot be seen
    /// cannot be asked.
    pub fn unlock(&self, settings: &AiSettings, feature: Feature) -> Result<String, AiError> {
        let key = self
            .keys
            .get(account(settings.provider))
            .ok_or(AiError::NoKey)?;
        let feature_on = match feature {
            Feature::WeekInWords => settings.week_in_words,
            Feature::TellTheSun => settings.tell_the_sun,
            Feature::DayInALine => settings.day_in_a_line,
            Feature::AskTheAlmanac => settings.ask_the_almanac,
        };
        if settings.enabled && feature_on && !settings.hidden {
            Ok(key)
        } else {
            Err(AiError::Off)
        }
    }

    fn client(&self) -> Result<reqwest::Client, AiError> {
        let mut slot = self.client.lock();
        if let Some(client) = slot.as_ref() {
            return Ok(client.clone());
        }
        // The same TLS the updater uses; whichever asks first sets it up.
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
        let client = reqwest::Client::builder()
            .timeout(TIMEOUT)
            .build()
            .map_err(|err| AiError::Failed(err.to_string()))?;
        *slot = Some(client.clone());
        Ok(client)
    }

    /// Asks the chosen service `ask` and returns its response body. The key
    /// goes in a header and nowhere else: it is not logged.
    pub async fn ask(&self, settings: &AiSettings, key: &str, ask: &Ask) -> Result<Value, AiError> {
        let provider = settings.provider;
        let url = ai::endpoint(provider, &settings.base_url)?;
        let model = ai::model_for(provider, &settings.model)?;
        let body = ai::body(provider, &model, ask);
        let request = self.client()?.post(url);
        let request = match provider {
            Provider::Anthropic => request
                .header("x-api-key", key)
                .header("anthropic-version", ai::ANTHROPIC_VERSION),
            Provider::OpenAi | Provider::Custom => request.bearer_auth(key),
        };
        let response = request.json(&body).send().await.map_err(|err| {
            log::debug!("ai request failed: {err}");
            AiError::Failed(if err.is_timeout() {
                "it took too long".to_owned()
            } else {
                "no connection".to_owned()
            })
        })?;
        let status = response.status();
        if !status.is_success() {
            log::warn!("the ai service answered {status}");
            return Err(AiError::from_status(status.as_u16()));
        }
        response.json().await.map_err(|err| {
            log::debug!("ai response was not JSON: {err}");
            AiError::Empty
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// Keys kept in memory.
    #[derive(Default)]
    struct MemoryKeys(Mutex<HashMap<String, String>>);

    impl KeyStore for MemoryKeys {
        fn get(&self, account: &str) -> Option<String> {
            self.0.lock().get(account).cloned()
        }

        fn set(&self, account: &str, key: Option<&str>) -> Result<(), String> {
            let mut keys = self.0.lock();
            match key {
                Some(key) => keys.insert(account.to_owned(), key.to_owned()),
                None => keys.remove(account),
            };
            Ok(())
        }
    }

    fn assistant() -> Assistant {
        Assistant::new(Box::new(MemoryKeys::default()))
    }

    fn on() -> AiSettings {
        AiSettings {
            enabled: true,
            ..AiSettings::default()
        }
    }

    const CLAUDE: Provider = Provider::Anthropic;

    #[test]
    fn it_is_locked_until_a_key_is_given_whatever_the_switches_say() {
        let ai = assistant();
        assert!(!ai.status(CLAUDE).has_key);
        assert_eq!(ai.unlock(&on(), Feature::WeekInWords), Err(AiError::NoKey));
    }

    #[test]
    fn a_key_alone_opens_nothing_ai_is_off_until_switched_on() {
        let ai = assistant();
        assert!(ai.set_key(CLAUDE, Some("  sk-ant-test  ")).unwrap().has_key);
        assert_eq!(
            ai.unlock(&AiSettings::default(), Feature::WeekInWords),
            Err(AiError::Off),
            "off by default"
        );
        assert_eq!(
            ai.unlock(&on(), Feature::WeekInWords).as_deref(),
            Ok("sk-ant-test"),
            "trimmed as it was stored"
        );
    }

    #[test]
    fn each_feature_has_a_switch_of_its_own() {
        let ai = assistant();
        ai.set_key(CLAUDE, Some("sk-ant-test")).unwrap();
        let settings = AiSettings {
            tell_the_sun: false,
            ..on()
        };
        assert!(ai.unlock(&settings, Feature::WeekInWords).is_ok());
        assert_eq!(ai.unlock(&settings, Feature::TellTheSun), Err(AiError::Off));
    }

    #[test]
    fn each_service_has_a_key_of_its_own_and_never_gets_anothers() {
        let ai = assistant();
        ai.set_key(Provider::Anthropic, Some("sk-ant-one")).unwrap();
        let openai = AiSettings {
            provider: Provider::OpenAi,
            ..on()
        };
        assert!(!ai.status(Provider::OpenAi).has_key);
        assert_eq!(
            ai.unlock(&openai, Feature::WeekInWords),
            Err(AiError::NoKey),
            "Claude's key is not sent to OpenAI"
        );
        ai.set_key(Provider::OpenAi, Some("sk-two")).unwrap();
        assert_eq!(
            ai.unlock(&openai, Feature::WeekInWords).as_deref(),
            Ok("sk-two")
        );
        assert_eq!(
            ai.unlock(&on(), Feature::WeekInWords).as_deref(),
            Ok("sk-ant-one")
        );
        let names = [
            account(Provider::Anthropic),
            account(Provider::OpenAi),
            account(Provider::Custom),
        ];
        assert!(names[0] != names[1] && names[1] != names[2] && names[0] != names[2]);
    }

    #[test]
    fn hidden_nothing_can_be_asked_whatever_else_is_on() {
        let ai = assistant();
        ai.set_key(CLAUDE, Some("sk-ant-test")).unwrap();
        let hidden = AiSettings {
            hidden: true,
            ..on()
        };
        for feature in [
            Feature::WeekInWords,
            Feature::TellTheSun,
            Feature::DayInALine,
            Feature::AskTheAlmanac,
        ] {
            assert_eq!(ai.unlock(&hidden, feature), Err(AiError::Off));
            assert!(ai.unlock(&on(), feature).is_ok());
        }
    }

    #[test]
    fn the_key_is_removed_by_none_or_by_nothing_but_spaces() {
        let ai = assistant();
        ai.set_key(CLAUDE, Some("sk-ant-test")).unwrap();
        assert!(!ai.set_key(CLAUDE, Some("   ")).unwrap().has_key);
        ai.set_key(CLAUDE, Some("sk-ant-test")).unwrap();
        assert!(!ai.set_key(CLAUDE, None).unwrap().has_key);
    }

    #[test]
    fn the_window_is_told_only_whether_there_is_a_key() {
        let ai = assistant();
        ai.set_key(CLAUDE, Some("sk-ant-secret")).unwrap();
        let json = serde_json::to_string(&ai.status(CLAUDE)).unwrap();
        assert_eq!(json, r#"{"has_key":true}"#);
    }
}
