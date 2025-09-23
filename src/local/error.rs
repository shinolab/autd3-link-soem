// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use std::{ffi::CString, time::Duration};

use autd3_core::link::LinkError;
use thiserror::Error;

use crate::local::inner::State;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum SOEMError {
    #[error("Cycle({0:?}) must be a multiple of 500μs and not 0")]
    InvalidCycle(Duration),
    #[error("No AUTD device was found")]
    NoDeviceFound,
    #[error("No socket connection on {0:?}")]
    NoSocketConnection(CString),
    #[error("The number of slaves you specified is {1}, but {0} devices are found")]
    SlaveNotFound(u16, u16),
    #[error("One ore more slaves are not responding")]
    NotResponding,
    #[error("One ore more slaves did not reach required state (expected: {0}, actual: {1})")]
    NotReachedRequiredState(State, State),
    #[error("Invalid interface name: {0}")]
    InvalidInterfaceName(String),
    #[error(
        "Failed to synchronize devices. Maximum system time difference ({0:?}) exceeded the tolerance ({1:?})"
    )]
    SynchronizeFailed(Duration, Duration),
    #[error("{0}")]
    ThreadPriorityError(#[from] thread_priority::Error),
    #[error("Failed to set CPU affinity for the EtherCAT thread: {0:?}")]
    AffinitySetFailed(core_affinity::CoreId),
}

impl From<SOEMError> for LinkError {
    fn from(val: SOEMError) -> LinkError {
        LinkError::new(val.to_string())
    }
}
