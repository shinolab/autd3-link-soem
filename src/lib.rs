#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::unescaped_backticks)]

//! This crate provides a link to AUTD using [SOEM](https://github.com/OpenEtherCATsociety/SOEM).

#[cfg_attr(docsrs, doc(cfg(feature = "local")))]
#[cfg(feature = "local")]
/// Using SOEM on the local machine.
pub mod local;
#[cfg_attr(docsrs, doc(cfg(all(feature = "local", target_os = "windows"))))]
#[cfg(all(feature = "local", target_os = "windows"))]
pub use local::ProcessPriority;
#[cfg_attr(docsrs, doc(cfg(feature = "local")))]
#[cfg(feature = "local")]
pub use local::{EthernetAdapters, SOEM, SOEMOption, Status, ThreadPriority, ThreadPriorityValue};

#[cfg_attr(docsrs, doc(cfg(feature = "remote")))]
#[cfg(feature = "remote")]
/// Using SOEM on a remote machine.
pub mod remote;
#[cfg_attr(docsrs, doc(cfg(feature = "remote")))]
#[cfg(feature = "remote")]
pub use remote::RemoteSOEM;
