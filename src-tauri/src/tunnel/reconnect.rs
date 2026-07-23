use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::model::ReconnectPolicy;

/// Should another reconnect attempt be made? `attempt` is 1-based (the first
/// retry is attempt 1).
pub fn should_retry(policy: &ReconnectPolicy, attempt: u32) -> bool {
    policy.enabled && policy.max_retries.is_none_or(|max| attempt <= max)
}

/// Exponential backoff delay for the given (1-based) attempt, capped at
/// `max_delay_ms` and optionally jittered (full jitter in `[delay/2, delay]`).
pub fn backoff_delay(policy: &ReconnectPolicy, attempt: u32) -> Duration {
    let exponent = attempt.saturating_sub(1) as i32;
    let base = policy.initial_delay_ms as f64 * policy.factor.powi(exponent);
    let capped = base.min(policy.max_delay_ms as f64).max(0.0);

    let ms = if policy.jitter {
        let half = capped / 2.0;
        half + (capped - half) * jitter_fraction()
    } else {
        capped
    };
    Duration::from_millis(ms as u64)
}

/// Cheap pseudo-random fraction in `[0, 1)` from the clock — good enough to
/// de-synchronize reconnect storms; not for cryptographic use.
fn jitter_fraction() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos % 1000) as f64 / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(enabled: bool, max_retries: Option<u32>) -> ReconnectPolicy {
        ReconnectPolicy {
            enabled,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            factor: 2.0,
            jitter: false,
            max_retries,
        }
    }

    #[test]
    fn retries_forever_when_unlimited() {
        let p = policy(true, None);
        assert!(should_retry(&p, 1));
        assert!(should_retry(&p, 1000));
    }

    #[test]
    fn stops_after_max_retries() {
        let p = policy(true, Some(3));
        assert!(should_retry(&p, 3));
        assert!(!should_retry(&p, 4));
    }

    #[test]
    fn never_retries_when_disabled() {
        let p = policy(false, None);
        assert!(!should_retry(&p, 1));
    }

    #[test]
    fn backoff_grows_exponentially_then_caps() {
        let p = policy(true, None);
        assert_eq!(backoff_delay(&p, 1), Duration::from_millis(1000));
        assert_eq!(backoff_delay(&p, 2), Duration::from_millis(2000));
        assert_eq!(backoff_delay(&p, 3), Duration::from_millis(4000));
        // Eventually hits the 60s cap and stays there.
        assert_eq!(backoff_delay(&p, 20), Duration::from_millis(60000));
    }
}
