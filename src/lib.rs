#[cfg(feature = "local")]
pub mod local;
#[cfg(all(feature = "local", target_os = "windows"))]
pub use local::ProcessPriority;
#[cfg(feature = "local")]
pub use local::{
    EthernetAdapters, Status, SyncMode, ThreadPriority, ThreadPriorityValue, TimerStrategy, SOEM,
};

#[cfg(feature = "remote")]
pub mod remote;
#[cfg(feature = "remote")]
pub use remote::RemoteSOEM;
