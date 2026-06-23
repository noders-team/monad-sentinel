use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Firing,
    ReNotify,
    Resolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Inactive,
    Pending { since_ms: i64 },
    Firing { last_notified_ms: i64 },
}

/// State machine for a single rule: pending(for) → firing → resolved, with re-notification.
#[derive(Debug)]
pub struct AlertState {
    for_ms: i64,
    repeat_ms: i64,
    phase: Phase,
}

impl AlertState {
    pub fn new(for_ms: i64, repeat_ms: i64) -> Self {
        Self { for_ms, repeat_ms, phase: Phase::Inactive }
    }

    /// Processes one tick. Returns the transition, if one occurred.
    pub fn step(&mut self, active: bool, now_ms: i64) -> Option<Transition> {
        match self.phase {
            Phase::Inactive => {
                if active {
                    if self.for_ms == 0 {
                        self.phase = Phase::Firing { last_notified_ms: now_ms };
                        Some(Transition::Firing)
                    } else {
                        self.phase = Phase::Pending { since_ms: now_ms };
                        None
                    }
                } else {
                    None
                }
            }
            Phase::Pending { since_ms } => {
                if !active {
                    self.phase = Phase::Inactive;
                    None
                } else if now_ms - since_ms >= self.for_ms {
                    self.phase = Phase::Firing { last_notified_ms: now_ms };
                    Some(Transition::Firing)
                } else {
                    None
                }
            }
            Phase::Firing { last_notified_ms } => {
                if !active {
                    self.phase = Phase::Inactive;
                    Some(Transition::Resolved)
                } else if now_ms - last_notified_ms >= self.repeat_ms {
                    self.phase = Phase::Firing { last_notified_ms: now_ms };
                    Some(Transition::ReNotify)
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fires_only_after_for_duration() {
        let mut a = AlertState::new(30_000, 900_000);
        assert_eq!(a.step(true, 0), None);           // pending
        assert_eq!(a.step(true, 29_000), None);      // still pending
        assert_eq!(a.step(true, 30_000), Some(Transition::Firing));
    }

    #[test]
    fn resolves_after_firing() {
        let mut a = AlertState::new(0, 900_000);
        assert_eq!(a.step(true, 0), Some(Transition::Firing));
        assert_eq!(a.step(false, 1_000), Some(Transition::Resolved));
    }

    #[test]
    fn renotifies_after_repeat_interval() {
        let mut a = AlertState::new(0, 900_000);
        assert_eq!(a.step(true, 0), Some(Transition::Firing));
        assert_eq!(a.step(true, 800_000), None);
        assert_eq!(a.step(true, 900_000), Some(Transition::ReNotify));
    }

    #[test]
    fn pending_then_inactive_no_transition() {
        let mut a = AlertState::new(30_000, 900_000);
        assert_eq!(a.step(true, 0), None);
        assert_eq!(a.step(false, 5_000), None);
    }
}
