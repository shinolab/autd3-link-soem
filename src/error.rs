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

use crate::inner::State;

#[derive(Debug)]
#[non_exhaustive]
pub enum SOEMError {
    InvalidCycle(Duration),
    NoDeviceFound,
    NoSocketConnection(CString),
    SlaveNotFound(u16, u16),
    NotResponding,
    NotReachedRequiredState(State, State),
    InvalidInterfaceName(String),
    SynchronizeFailed(Duration, Duration),
    ThreadPriorityError(thread_priority::Error),
    AffinitySetFailed(core_affinity::CoreId),
    Io(std::io::Error),
}

impl std::fmt::Display for SOEMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SOEMError::InvalidCycle(duration) => {
                write!(
                    f,
                    "Cycle({:?}) must be a multiple of 500μs and not 0",
                    duration
                )
            }
            SOEMError::NoDeviceFound => write!(f, "No AUTD device was found"),
            SOEMError::NoSocketConnection(name) => {
                write!(f, "No socket connection on {:?}", name)
            }
            SOEMError::SlaveNotFound(found, expected) => {
                write!(
                    f,
                    "The number of slaves you specified is {}, but {} devices are found",
                    expected, found
                )
            }
            SOEMError::NotResponding => write!(f, "One ore more slaves are not responding"),
            SOEMError::NotReachedRequiredState(expected, actual) => {
                write!(
                    f,
                    "One ore more slaves did not reach required state (expected: {}, actual: {})",
                    expected, actual
                )
            }
            SOEMError::InvalidInterfaceName(name) => {
                write!(f, "Invalid interface name: {}", name)
            }
            SOEMError::SynchronizeFailed(max_diff, tolerance) => {
                write!(
                    f,
                    "Failed to synchronize devices. Maximum system time difference ({:?}) exceeded the tolerance ({:?})",
                    max_diff, tolerance
                )
            }
            SOEMError::ThreadPriorityError(err) => write!(f, "{}", err),
            SOEMError::AffinitySetFailed(core_id) => {
                write!(
                    f,
                    "Failed to set CPU affinity for the EtherCAT thread: {:?}",
                    core_id
                )
            }
            SOEMError::Io(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for SOEMError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SOEMError::ThreadPriorityError(err) => Some(err),
            SOEMError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<thread_priority::Error> for SOEMError {
    fn from(err: thread_priority::Error) -> Self {
        SOEMError::ThreadPriorityError(err)
    }
}

impl From<std::io::Error> for SOEMError {
    fn from(err: std::io::Error) -> Self {
        SOEMError::Io(err)
    }
}

impl From<SOEMError> for LinkError {
    fn from(val: SOEMError) -> LinkError {
        LinkError::new(val.to_string())
    }
}
