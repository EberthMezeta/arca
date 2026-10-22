use std::sync::OnceLock;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;

    /// Tiempo transcurrido desde un punto fijo; nunca retrocede aunque cambie la hora del sistema.
    fn monotonic_now(&self) -> Duration;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn monotonic_now(&self) -> Duration {
        static PROCESS_START: OnceLock<Instant> = OnceLock::new();
        PROCESS_START.get_or_init(Instant::now).elapsed()
    }
}
