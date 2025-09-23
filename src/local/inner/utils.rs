// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use std::ffi::{CStr, CString};

use crate::{
    EthernetAdapters,
    local::{
        error::SOEMError,
        inner::{Context, ec_slave},
    },
};

pub fn is_autd3(slave: &ec_slave) -> bool {
    const AUTD_NAME: &CStr = c"AUTD";
    let name = unsafe { std::ffi::CStr::from_ptr(slave.name.as_ptr()) };
    tracing::trace!("Slave name: {:?}", name);
    name == AUTD_NAME
}

pub fn lookup_autd() -> Result<CString, SOEMError> {
    let adapters = EthernetAdapters::new();
    tracing::debug!("Found {} network adapters.", adapters.len());
    adapters
        .into_iter()
        .find_map(|adapter| {
            let ifname = match CString::new(adapter.name().to_owned()) {
                Ok(ifname) => ifname,
                Err(_) => return None,
            };
            tracing::debug!("Searching AUTD device on {}.", adapter.name());
            let ctx = Context::new();
            match ctx.init(ifname.clone()) {
                Ok(_) => {
                    if let Some(wc) = ctx.config_init() {
                        tracing::trace!("Found {} slaves on {}.", wc, adapter.name());
                        ctx.slaves().all(is_autd3).then_some(ifname)
                    } else {
                        tracing::trace!("No slave found on {}.", adapter.name());
                        None
                    }
                }
                Err(_) => {
                    tracing::trace!("Failed to initialize SOEM on {}.", adapter.name());
                    None
                }
            }
        })
        .ok_or(SOEMError::NoDeviceFound)
}
