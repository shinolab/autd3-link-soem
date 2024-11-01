#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(u8)]
pub enum TimerStrategy {
    SpinSleep = 0,
    StdSleep = 1,
    SpinWait = 2,
}
