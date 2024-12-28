#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg_attr(docsrs, doc(cfg(feature = "local")))]
#[cfg(feature = "local")]
pub mod local;
#[cfg_attr(docsrs, doc(cfg(all(feature = "local", target_os = "windows"))))]
#[cfg(all(feature = "local", target_os = "windows"))]
pub use local::ProcessPriority;
#[cfg_attr(docsrs, doc(cfg(feature = "local")))]
#[cfg(feature = "local")]
pub use local::{
    EthernetAdapters, Status, SyncMode, ThreadPriority, ThreadPriorityValue, TimerStrategy, SOEM,
};

#[cfg_attr(docsrs, doc(cfg(feature = "remote")))]
#[cfg(feature = "remote")]
pub mod remote;
#[cfg_attr(docsrs, doc(cfg(feature = "remote")))]
#[cfg(feature = "remote")]
pub use remote::RemoteSOEM;
