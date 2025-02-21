use std::time::Duration;

use autd3_core::link::LinkError;
use thiserror::Error;

use super::state::EcStatus;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SOEMError {
    #[error("Cycle({0:?}) must be a multiple of 500μs and not 0")]
    InvalidCycle(Duration),
    #[error("No AUTD device was found")]
    NoDeviceFound,
    #[error("No socket connection on {0}")]
    NoSocketConnection(String),
    #[error("The number of slaves you specified is {1}, but {0} devices are found")]
    SlaveNotFound(u16, u16),
    #[error("One ore more slaves are not responding")]
    NotResponding(EcStatus),
    #[error("One ore more slaves did not reach safe operational state: {0}")]
    NotReachedSafeOp(u16),
    #[error("Invalid interface name: {0}")]
    InvalidInterfaceName(String),
    #[error(
        "Failed to synchronize devices. Maximum system time difference ({0:?}) exceeded the tolerance ({1:?})"
    )]
    SynchronizeFailed(Duration, Duration),
    #[error("{0}")]
    ThreadPriorityError(#[from] thread_priority::Error),
}

impl From<SOEMError> for LinkError {
    fn from(val: SOEMError) -> LinkError {
        LinkError::new(val.to_string())
    }
}
