// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use crate::inner;

use std::ffi::CStr;

#[derive(Clone, Debug)]
pub struct EthernetAdapter {
    desc: String,
    name: String,
}

impl EthernetAdapter {
    /// The description of the adapter.
    pub fn desc(&self) -> &str {
        &self.desc
    }

    /// The name of the adapter.
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl std::fmt::Display for EthernetAdapter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}", self.name, self.desc)
    }
}

/// A list of Ethernet adapters.
#[derive(Clone)]
pub struct EthernetAdapters {
    adapters: Vec<EthernetAdapter>,
}

impl std::ops::Deref for EthernetAdapters {
    type Target = [EthernetAdapter];

    fn deref(&self) -> &Self::Target {
        &self.adapters
    }
}

impl std::iter::IntoIterator for EthernetAdapters {
    type Item = EthernetAdapter;
    type IntoIter = std::vec::IntoIter<EthernetAdapter>;

    fn into_iter(self) -> Self::IntoIter {
        self.adapters.into_iter()
    }
}

impl EthernetAdapters {
    /// Create a new [`EthernetAdapters`].
    pub fn new() -> Self {
        let mut adapters = Vec::new();
        unsafe {
            let head = inner::ec_find_adapters();
            let mut adapter = head;
            while !adapter.is_null() {
                if let Ok(name) = CStr::from_ptr(((*adapter).name).as_ptr()).to_str() {
                    adapters.push(EthernetAdapter {
                        desc: CStr::from_ptr(((*adapter).desc).as_ptr())
                            .to_str()
                            .unwrap_or("")
                            .to_string(),
                        name: name.to_string(),
                    });
                }
                adapter = (*adapter).next;
            }
            inner::ec_free_adapters(head);
            EthernetAdapters { adapters }
        }
    }
}

impl Default for EthernetAdapters {
    fn default() -> Self {
        Self::new()
    }
}
