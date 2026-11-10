//! Claude, behind three locks: a key, the master switch and the feature's
//! own switch. This holds the key (in the OS credential store, never in a
//! file of ours and never handed back to the window) and makes the request.
//!
//! The app asks Claude nothing on its own. Every request here begins with a
//! click on something that says what will be sent.

use std::time::Duration;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::ai::{self, AiError};
use crate::core::settings::AiSettings;

/// The Messages API.
const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

/// Where the key is kept in the OS credential store.
const SERVICE: &str = "dev.bajpai.keepmeawake";
const ACCOUNT: &str = "anthropic-api-key";

/// How long one request may take. Thinking is always on, and a busy day at
/// Anthropic is slow rather than failed.
const TIMEOUT: Duration = Duration::from_secs(120);

/// What the window may know about the key: whether there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiStatus {
    /// A key is stored.
    pub has_key: bool,
}

/// Somewhere a secret can be kept.
pub trait KeyStore: Send + Sync {
    /// The key, if one is stored.
    fn get(&self) -> Option<String>;
    /// Stores `key`, or removes it with `None`.
    fn set(&self, key: Option<&str>) -> Result<(), String>;
}

/// The OS credential store: Credential Manager on Windows, the Keychain on
/// a Mac.
#[derive(Debug, Default, Clone, Copy)]
pub struct OsKeyStore;

impl OsKeyStore {
    fn entry() -> Result<keyring::Entry, String> {
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|err| err.to_string())
    }
}

impl KeyStore for OsKeyStore {
    fn get(&self) -> Option<String> {
        let key = Self::entry().ok()?.get_password().ok()?;
        Some(key).filter(|key| !key.trim().is_empty())
    }

    fn set(&self, key: Option<&str>) -> Result<(), String> {
        let entry = Self::entry()?;
        match key {
            Some(key) => entry.set_password(key).map_err(|err| err.to_string()),
            None => match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(err) => Err(err.to_string()),
            },
        }
    }
}

/// Which of Claude's features is being asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Feature {
    /// The Almanac's week, in words.
    WeekInWords,
    /// Home's sentence, turned into a plan.
    TellTheSun,
}

/// The key and the client that asks.
pub struct Claude {
    keys: Box<dyn KeyStore>,
    /// Built on first use: most runs of the app never ask.
    client: Mutex<Option<reqwest::Client>>,
}

impl Claude {
    /// Claude with its key in `keys`.
    pub fn new(keys: Box<dyn KeyStore>) -> Self {
        Self {
            keys,
            client: Mutex::new(None),
        }
    }

    /// Whether a key is stored.
    pub fn status(&self) -> AiStatus {
        AiStatus {
            has_key: self.keys.get().is_some(),
        }
    }

    /// Stores the key, or removes it with `None` or nothing but spaces.
    pub fn set_key(&self, key: Option<&str>) -> Result<AiStatus, String> {
        let key = key.map(str::trim).filter(|key| !key.is_empty());
        self.keys.set(key)?;
        Ok(self.status())
    }

    /// The key, if all three locks are open for `feature`: a key is stored,
    /// Claude is switched on, and so is the feature.
    pub fn unlock(&self, settings: &AiSettings, feature: Feature) -> Result<String, AiError> {
        let key = self.keys.get().ok_or(AiError::NoKey)?;
        let feature_on = match feature {
            Feature::WeekInWords => settings.week_in_words,
            Feature::TellTheSun => settings.tell_the_sun,
        };
        if settings.enabled && feature_on {
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

    /// Sends `body` to the Messages API with `key` and returns the response
    /// body. The key goes in a header and nowhere else: it is not logged.
    pub async fn ask(&self, key: &str, body: &Value) -> Result<Value, AiError> {
        let response = self
            .client()?
            .post(ENDPOINT)
            .header("x-api-key", key)
            .header("anthropic-version", ai::API_VERSION)
            .header("anthropic-beta", ai::FALLBACK_BETA)
            .json(body)
            .send()
            .await
            .map_err(|err| {
                log::debug!("claude request failed: {err}");
                AiError::Failed(if err.is_timeout() {
                    "it took too long".to_owned()
                } else {
                    "no connection".to_owned()
                })
            })?;
        let status = response.status();
        if !status.is_success() {
            log::warn!("claude answered {status}");
            return Err(AiError::from_status(status.as_u16()));
        }
        response.json().await.map_err(|err| {
            log::debug!("claude response was not JSON: {err}");
            AiError::Empty
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key kept in memory.
    #[derive(Default)]
    struct MemoryKeys(Mutex<Option<String>>);

    impl KeyStore for MemoryKeys {
        fn get(&self) -> Option<String> {
            self.0.lock().clone()
        }

        fn set(&self, key: Option<&str>) -> Result<(), String> {
            *self.0.lock() = key.map(str::to_owned);
            Ok(())
        }
    }

    fn claude() -> Claude {
        Claude::new(Box::new(MemoryKeys::default()))
    }

    fn on() -> AiSettings {
        AiSettings {
            enabled: true,
            ..AiSettings::default()
        }
    }

    #[test]
    fn it_is_locked_until_a_key_is_given_whatever_the_switches_say() {
        let claude = claude();
        assert!(!claude.status().has_key);
        assert_eq!(
            claude.unlock(&on(), Feature::WeekInWords),
            Err(AiError::NoKey)
        );
    }

    #[test]
    fn a_key_alone_opens_nothing_claude_is_off_until_switched_on() {
        let claude = claude();
        assert!(claude.set_key(Some("  sk-ant-test  ")).unwrap().has_key);
        assert_eq!(
            claude.unlock(&AiSettings::default(), Feature::WeekInWords),
            Err(AiError::Off),
            "off by default"
        );
        assert_eq!(
            claude.unlock(&on(), Feature::WeekInWords).as_deref(),
            Ok("sk-ant-test"),
            "trimmed as it was stored"
        );
    }

    #[test]
    fn each_feature_has_a_switch_of_its_own() {
        let claude = claude();
        claude.set_key(Some("sk-ant-test")).unwrap();
        let settings = AiSettings {
            tell_the_sun: false,
            ..on()
        };
        assert!(claude.unlock(&settings, Feature::WeekInWords).is_ok());
        assert_eq!(
            claude.unlock(&settings, Feature::TellTheSun),
            Err(AiError::Off)
        );
    }

    #[test]
    fn the_key_is_removed_by_none_or_by_nothing_but_spaces() {
        let claude = claude();
        claude.set_key(Some("sk-ant-test")).unwrap();
        assert!(!claude.set_key(Some("   ")).unwrap().has_key);
        claude.set_key(Some("sk-ant-test")).unwrap();
        assert!(!claude.set_key(None).unwrap().has_key);
    }

    #[test]
    fn the_window_is_told_only_whether_there_is_a_key() {
        let claude = claude();
        claude.set_key(Some("sk-ant-secret")).unwrap();
        let json = serde_json::to_string(&claude.status()).unwrap();
        assert_eq!(json, r#"{"has_key":true}"#);
    }
}
