// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

mod context;
mod ethernet_adapters;
mod handler;
mod iomap;
mod option;
mod smoothing;
mod soem_bindings;
mod state;
mod status;
mod utils;

pub use context::*;
pub use ethernet_adapters::EthernetAdapters;
pub use handler::SOEMHandler;
pub use option::{SOEMOption, SOEMOptionFull};
pub use soem_bindings::*;
pub use state::State;
pub use status::Status;

pub mod consts {
    pub const EC_TIMEOUTSTATE: u32 = super::soem_bindings::EC_TIMEOUTSTATE;
    pub const EC_TIMEOUTRET: u32 = super::soem_bindings::EC_TIMEOUTRET;

    pub const ECT_REG_DCSYSTIME: u16 = super::soem_bindings::ECT_REG_DCSYSTIME as _;
    pub const ECT_REG_DCSYSDIFF: u16 = super::soem_bindings::ECT_REG_DCSYSDIFF as _;
}
