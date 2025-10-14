// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use std::{num::NonZeroUsize, time::Duration};

use autd3_core::ethercat::EC_CYCLE_TIME_BASE;

use thread_priority::{ThreadBuilder, ThreadPriority};

use crate::inner::option::SOEMOptionFull;

/// A option for [`SOEM`].
///
/// [`SOEM`]: crate::link_soem::SOEM
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SOEMOption {
    /// The network interface name. If `None`, the network interface will be automatically selected to which the AUTD3 device is connected. The default is `None`.
    pub ifname: Option<String>,
    /// The interval to check the state. The default is 100ms.
    pub state_check_interval: Duration,
    /// The cycle of the sync0 signal. The value must be a multiple of [`EC_CYCLE_TIME_BASE`] and not be zero. The default is 1ms.
    pub sync0_cycle: Duration,
    /// The synchronization tolerance. The default is 1us.
    pub sync_tolerance: Duration,
    /// The synchronization timeout. The default is 10s.
    pub sync_timeout: Duration,
}

impl Default for SOEMOption {
    fn default() -> Self {
        Self {
            ifname: None,
            state_check_interval: Duration::from_millis(100),
            sync0_cycle: EC_CYCLE_TIME_BASE * 2,
            sync_tolerance: std::time::Duration::from_micros(1),
            sync_timeout: std::time::Duration::from_secs(10),
        }
    }
}

impl From<SOEMOption> for SOEMOptionFull {
    fn from(value: SOEMOption) -> Self {
        Self {
            buf_size: NonZeroUsize::new(16).unwrap(),
            ifname: value.ifname,
            state_check_interval: value.state_check_interval,
            sync0_cycle: value.sync0_cycle,
            send_cycle: value.sync0_cycle,
            thread_builder: ThreadBuilder::default().name("tx-rx-thread").priority(
                ThreadPriority::Os(thread_priority::ThreadPriorityOsValue::from(
                    thread_priority::WinAPIThreadPriority::TimeCritical,
                )),
            ),
            sync_tolerance: value.sync_tolerance,
            sync_timeout: value.sync_timeout,
            affinity: None,
        }
    }
}
