use autd3_core::utils::timer::TimerResolutionGuard;
use std::{num::NonZeroU32, time::Duration};

pub(crate) trait Sleep {
    fn sleep(duration: Duration);
}

pub(crate) struct StdSleep {}

impl Sleep for StdSleep {
    fn sleep(duration: Duration) {
        let _timer_guard = TimerResolutionGuard::new(Some(NonZeroU32::MIN));
        std::thread::sleep(duration);
    }
}

pub(crate) struct SpinSleep {}

impl Sleep for SpinSleep {
    fn sleep(duration: Duration) {
        spin_sleep::sleep(duration);
    }
}

pub(crate) struct SpinWait {}

impl Sleep for SpinWait {
    fn sleep(duration: Duration) {
        let expired = std::time::Instant::now() + duration;
        while std::time::Instant::now() < expired {
            std::hint::spin_loop();
        }
    }
}
