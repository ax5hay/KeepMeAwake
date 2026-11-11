//! Requests the UI (or tray) can make to change what the engine is doing.

use serde::{Deserialize, Serialize};

/// What the user asked for. This is the only mutation the UI can make to the
/// session; everything else is derived from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanRequest {
    /// Let the laptop sleep normally.
    Off,
    /// Stay awake until told otherwise.
    Indefinite,
    /// Stay awake for a fixed amount of time from now.
    For {
        /// Length of the session in whole seconds.
        seconds: u64,
    },
    /// Stay awake until a wall-clock moment, expressed as Unix milliseconds.
    Until {
        /// Target moment as Unix epoch milliseconds (UTC).
        unix_ms: u64,
    },
}

impl PlanRequest {
    /// Converts an `Until` request into a `For` request relative to `now_unix_ms`.
    /// Other variants are returned unchanged. A moment in the past becomes `Off`.
    #[must_use]
    pub fn resolve(self, now_unix_ms: u64) -> PlanRequest {
        match self {
            PlanRequest::Until { unix_ms } if unix_ms <= now_unix_ms => PlanRequest::Off,
            // Rounded up: a deadline a few milliseconds short of the minute
            // asked for would read as the minute before.
            PlanRequest::Until { unix_ms } => PlanRequest::For {
                seconds: (unix_ms - now_unix_ms).div_ceil(1000).max(1),
            },
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn until_in_future_becomes_for() {
        let req = PlanRequest::Until { unix_ms: 10_000 }.resolve(4_000);
        assert_eq!(req, PlanRequest::For { seconds: 6 });
    }

    #[test]
    fn until_rounds_up_to_the_second_asked_for() {
        let req = PlanRequest::Until { unix_ms: 60_000 }.resolve(1_250);
        assert_eq!(req, PlanRequest::For { seconds: 59 });
    }

    #[test]
    fn until_in_past_becomes_off() {
        assert_eq!(
            PlanRequest::Until { unix_ms: 1 }.resolve(2),
            PlanRequest::Off
        );
        assert_eq!(
            PlanRequest::Until { unix_ms: 2 }.resolve(2),
            PlanRequest::Off
        );
    }

    #[test]
    fn until_very_soon_rounds_up_to_one_second() {
        let req = PlanRequest::Until { unix_ms: 2_100 }.resolve(2_000);
        assert_eq!(req, PlanRequest::For { seconds: 1 });
    }

    #[test]
    fn other_variants_pass_through() {
        assert_eq!(PlanRequest::Off.resolve(0), PlanRequest::Off);
        assert_eq!(PlanRequest::Indefinite.resolve(0), PlanRequest::Indefinite);
        assert_eq!(
            PlanRequest::For { seconds: 5 }.resolve(0),
            PlanRequest::For { seconds: 5 }
        );
    }

    #[test]
    fn plan_request_serialises_with_kind_tag() {
        let json = serde_json::to_string(&PlanRequest::For { seconds: 60 }).unwrap();
        assert_eq!(json, r#"{"kind":"for","seconds":60}"#);
        let back: PlanRequest = serde_json::from_str(r#"{"kind":"indefinite"}"#).unwrap();
        assert_eq!(back, PlanRequest::Indefinite);
    }
}
