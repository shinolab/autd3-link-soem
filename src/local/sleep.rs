use std::{num::NonZeroU32, time::Duration};

#[doc(hidden)]
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub struct TimerResolutionGurad {
    timer_resolution: Option<NonZeroU32>,
}

impl TimerResolutionGurad {
    pub fn new(timer_resolution: Option<NonZeroU32>) -> Self {
        #[cfg(target_os = "windows")]
        timer_resolution.map(|timer_resolution| unsafe {
            windows::Win32::Media::timeBeginPeriod(timer_resolution.get())
        });
        Self { timer_resolution }
    }
}

impl Drop for TimerResolutionGurad {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        self.timer_resolution.map(|timer_resolution| unsafe {
            windows::Win32::Media::timeEndPeriod(timer_resolution.get())
        });
    }
}

pub(crate) trait Sleep {
    fn sleep(duration: Duration);
}

pub(crate) struct StdSleep {}

impl Sleep for StdSleep {
    fn sleep(duration: Duration) {
        let _timer_guard = TimerResolutionGurad::new(Some(NonZeroU32::MIN));
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
