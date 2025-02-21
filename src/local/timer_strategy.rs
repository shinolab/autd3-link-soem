/// The timer strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum TimerStrategy {
    /// Using [`spin_sleep`] crate.
    ///
    /// [`spin_sleep`]: https://docs.rs/spin_sleep
    SpinSleep = 0,
    /// Using [`std::thread::sleep`] function.
    StdSleep = 1,
    /// Using spin loop, or busy-wait loop.
    SpinWait = 2,
}
