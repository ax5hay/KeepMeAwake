//! Writes the engine's session events into the history log.

use crate::core::history::History;
use crate::engine::EngineEvent;

/// Applies `event` to `history`. Returns `true` when the log should be saved
/// now: as a stretch starts, so one cut short by sleep or a kill is still on
/// disk, and as it ends. Pauses are kept in memory until then.
pub fn record(history: &mut History, event: &EngineEvent, now_unix_ms: u64) -> bool {
    match *event {
        EngineEvent::SessionStarted {
            mode,
            paused,
            at_unix_ms,
        } => {
            history.open(at_unix_ms, mode);
            if let Some(reason) = paused {
                history.pause(at_unix_ms, reason);
            }
            true
        }
        EngineEvent::Paused(reason, _) => {
            history.pause(now_unix_ms, reason);
            false
        }
        EngineEvent::Resumed => {
            history.resume(now_unix_ms);
            false
        }
        EngineEvent::SessionEnded { ended, at_unix_ms } => {
            history.close(at_unix_ms, ended);
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::history::{Ended, Pause};
    use crate::core::{BatteryState, Mode, PauseReason};

    #[test]
    fn a_stretch_with_a_pause_is_logged_and_saved_at_both_ends() {
        let mut h = History::default();
        let started = EngineEvent::SessionStarted {
            mode: Mode::Indefinite,
            paused: None,
            at_unix_ms: 1_000,
        };
        assert!(record(&mut h, &started, 1_000));
        let paused = EngineEvent::Paused(PauseReason::OnBattery, BatteryState::default());
        assert!(!record(&mut h, &paused, 2_000));
        assert!(!record(&mut h, &EngineEvent::Resumed, 3_000));
        let ended = EngineEvent::SessionEnded {
            ended: Ended::User,
            at_unix_ms: 4_000,
        };
        assert!(record(&mut h, &ended, 4_100));
        let entry = &h.entries[0];
        assert_eq!(entry.started_unix_ms, 1_000);
        assert_eq!(
            entry.ended_unix_ms,
            Some(4_000),
            "the engine's time, not ours"
        );
        assert_eq!(entry.ended, Some(Ended::User));
        assert_eq!(
            entry.pauses,
            vec![Pause {
                from_unix_ms: 2_000,
                to_unix_ms: Some(3_000),
                reason: PauseReason::OnBattery,
            }]
        );
    }

    #[test]
    fn a_stretch_that_starts_paused_opens_with_its_pause() {
        let mut h = History::default();
        let started = EngineEvent::SessionStarted {
            mode: Mode::Timed,
            paused: Some(PauseReason::LowBattery),
            at_unix_ms: 1_000,
        };
        record(&mut h, &started, 1_000);
        assert_eq!(h.entries[0].pauses[0].from_unix_ms, 1_000);
        assert_eq!(h.entries[0].pauses[0].reason, PauseReason::LowBattery);
    }

    #[test]
    fn other_events_leave_the_log_alone() {
        let mut h = History::default();
        assert!(!record(
            &mut h,
            &EngineEvent::TimerEnded { plan_secs: 60 },
            1
        ));
        assert!(!record(
            &mut h,
            &EngineEvent::ScheduleStarted { ends_at_unix_ms: 2 },
            1
        ));
        assert_eq!(h, History::default());
    }
}
