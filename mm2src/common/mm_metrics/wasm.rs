use super::*;
use crate::now_ms;

/// Increment counter if an MmArc is not dropped yet and metrics system is initialized already.
#[macro_export]
macro_rules! mm_counter {
    ($metrics:expr, $name:expr, $value:expr) => {};
    ($metrics:expr, $name:expr, $value:expr, $($labels:tt)*) => {};
}

/// Update gauge if an MmArc is not dropped yet and metrics system is initialized already.
#[macro_export]
macro_rules! mm_gauge {
    ($_metrics:expr, $_name:expr, $_value:expr) => {};
    ($_metrics:expr, $_name:expr, $_value:expr, $($_labels:tt)*) => {};
}

/// Pass new timing value if an MmArc is not dropped yet and metrics system is initialized already.
#[macro_export]
macro_rules! mm_timing {
    ($_metrics:expr, $_name:expr, $_start:expr, $_end:expr) => {};
    ($_metrics:expr, $_name:expr, $_start:expr, $_end:expr, $($_labels:tt)*) => {};
}

#[derive(Default)]
pub struct Clock {}

impl ClockOps for Clock {
    fn now(&self) -> u64 { now_ms() }
}

#[derive(Default)]
pub struct Metrics {}

impl MetricsOps for Metrics {
    fn init(&self) -> Result<(), String> { Ok(()) }

    fn init_with_dashboard(&self, _log_state: LogWeak, _record_interval: f64) -> Result<(), String> { Ok(()) }

    fn clock(&self) -> Result<Clock, String> { Ok(Clock::default()) }

    fn collect_json(&self) -> Result<Json, String> { Ok(Json::default()) }
}
