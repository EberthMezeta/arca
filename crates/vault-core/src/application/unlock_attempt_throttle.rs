use chrono::{DateTime, Duration, Utc};

const FREE_FAILED_ATTEMPTS: u32 = 3;
const MAXIMUM_DELAY_SECONDS: i64 = 5 * 60;

/// Retardo exponencial tras intentos fallidos de desbloqueo (1 s, 2 s, 4 s ... hasta 5 min).
#[derive(Debug, Default)]
pub struct UnlockAttemptThrottle {
    failed_unlock_attempt_count: u32,
    next_attempt_allowed_at: Option<DateTime<Utc>>,
}

impl UnlockAttemptThrottle {
    /// Segundos que faltan para poder reintentar, o `None` si ya se puede.
    pub fn seconds_until_next_attempt(&self, now: DateTime<Utc>) -> Option<u64> {
        let next_attempt_allowed_at = self.next_attempt_allowed_at?;
        let remaining_seconds = (next_attempt_allowed_at - now).num_seconds();
        u64::try_from(remaining_seconds).ok().filter(|remaining_seconds| *remaining_seconds > 0)
    }

    pub fn register_failed_attempt(&mut self, now: DateTime<Utc>) {
        self.failed_unlock_attempt_count += 1;
        let Some(penalized_attempt_number) = self.failed_unlock_attempt_count.checked_sub(FREE_FAILED_ATTEMPTS) else {
            return;
        };
        let delay_seconds = 2_i64.saturating_pow(penalized_attempt_number).min(MAXIMUM_DELAY_SECONDS);
        self.next_attempt_allowed_at = Some(now + Duration::seconds(delay_seconds));
    }

    pub fn register_successful_attempt(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_failures_are_free_then_delay_grows() {
        let now = Utc::now();
        let mut unlock_attempt_throttle = UnlockAttemptThrottle::default();

        unlock_attempt_throttle.register_failed_attempt(now);
        unlock_attempt_throttle.register_failed_attempt(now);
        assert_eq!(unlock_attempt_throttle.seconds_until_next_attempt(now), None);

        unlock_attempt_throttle.register_failed_attempt(now);
        assert_eq!(unlock_attempt_throttle.seconds_until_next_attempt(now), Some(1));

        unlock_attempt_throttle.register_failed_attempt(now);
        assert_eq!(unlock_attempt_throttle.seconds_until_next_attempt(now), Some(2));
    }

    #[test]
    fn success_resets_the_counter() {
        let now = Utc::now();
        let mut unlock_attempt_throttle = UnlockAttemptThrottle::default();
        (0..10).for_each(|_| unlock_attempt_throttle.register_failed_attempt(now));

        unlock_attempt_throttle.register_successful_attempt();

        assert_eq!(unlock_attempt_throttle.seconds_until_next_attempt(now), None);
    }
}
