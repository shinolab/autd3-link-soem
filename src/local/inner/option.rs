// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use std::{ffi::CString, num::NonZeroUsize, time::Duration};

use autd3_core::ethercat::EC_CYCLE_TIME_BASE;

use thread_priority::ThreadPriority;

use crate::local::error::SOEMError;

/// A option for [`SOEM`].
///
/// [`SOEM`]: crate::local::link_soem::SOEM
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SOEMOption {
    /// The size of the send queue buffer. The default is 16.
    pub buf_size: NonZeroUsize,
    /// The network interface name. If `None`, the network interface will be automatically selected to which the AUTD3 device is connected. The default is `None`.
    pub ifname: Option<String>,
    /// The interval to check the state. The default is 100ms.
    pub state_check_interval: Duration,
    /// The cycle of the sync0 signal. The value must be a multiple of [`EC_CYCLE_TIME_BASE`] and not be zero. The default is 1ms.
    pub sync0_cycle: Duration,
    /// The send cycle. The value must be a multiple of [`EC_CYCLE_TIME_BASE`] and not be zero. The default is 1ms.
    pub send_cycle: Duration,
    /// The thread priority. The default is [`ThreadPriority::Max`].
    pub thread_priority: ThreadPriority,
    /// The synchronization tolerance. The default is 1us.
    pub sync_tolerance: Duration,
    /// The synchronization timeout. The default is 10s.
    pub sync_timeout: Duration,
    /// CPU affinity for the EtherCAT thread. The default is `None`, which means no affinity is set.
    pub affinity: Option<core_affinity::CoreId>,
}

impl Default for SOEMOption {
    fn default() -> Self {
        Self {
            buf_size: NonZeroUsize::new(16).unwrap(),
            ifname: None,
            state_check_interval: Duration::from_millis(100),
            sync0_cycle: EC_CYCLE_TIME_BASE * 2,
            send_cycle: EC_CYCLE_TIME_BASE * 2,
            thread_priority: ThreadPriority::Max,
            sync_tolerance: std::time::Duration::from_micros(1),
            sync_timeout: std::time::Duration::from_secs(10),
            affinity: None,
        }
    }
}

impl SOEMOption {
    pub(crate) fn validate(&self) -> Result<(), SOEMError> {
        if self.sync0_cycle.is_zero()
            || !self
                .sync0_cycle
                .as_nanos()
                .is_multiple_of(EC_CYCLE_TIME_BASE.as_nanos())
        {
            return Err(SOEMError::InvalidCycle(self.sync0_cycle));
        }
        if self.send_cycle.is_zero()
            || !self
                .send_cycle
                .as_nanos()
                .is_multiple_of(EC_CYCLE_TIME_BASE.as_nanos())
        {
            return Err(SOEMError::InvalidCycle(self.send_cycle));
        }
        Ok(())
    }

    pub(crate) fn ifname(&self) -> Result<CString, SOEMError> {
        self.ifname.as_ref().map_or_else(
            || {
                tracing::info!("No interface name is specified. Looking for AUTD device...");
                let ifname = crate::local::inner::utils::lookup_autd()?;
                tracing::info!("Found AUTD device on {:?}.", ifname);
                Ok(ifname)
            },
            |ifname| {
                CString::new(ifname.as_str())
                    .map_err(|_| SOEMError::InvalidInterfaceName(ifname.clone()))
            },
        )
    }
}
