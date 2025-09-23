// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
/// The status of the EtherCAT slave.
pub enum Status {
    /// The slave is in SAFE_OP + ERROR.
    Error = 0,
    /// The slave is lost.
    Lost = 1,
    /// The slave is in SAFE_OP.
    StateChanged = 2,
    /// The slave recovered.
    Recovered = 3,
    /// All slaves resumed OPERATIONAL.
    Resumed = 4,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Error => write!(f, "slave is in SAFE_OP + ERROR, attempting ack"),
            Status::Lost => write!(f, "slave is lost"),
            Status::StateChanged => write!(f, "slave is in SAFE_OP, change to OPERATIONAL"),
            Status::Recovered => write!(f, "slave is recovered"),
            Status::Resumed => write!(f, "all slaves resumed OPERATIONAL"),
        }
    }
}
