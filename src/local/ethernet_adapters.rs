use crate::local::soem_bindings;

use std::ffi::CStr;

use derive_more::{Deref, Display, IntoIterator};
use getset::Getters;

#[derive(Clone, Display, Getters)]
#[display("{}, {}", name, desc)]
pub struct EthernetAdapter {
    #[getset(get = "pub")]
    desc: String,
    #[getset(get = "pub")]
    name: String,
}

/// A list of Ethernet adapters.
#[derive(Clone, Deref, IntoIterator)]
pub struct EthernetAdapters {
    #[deref]
    #[into_iterator]
    adapters: Vec<EthernetAdapter>,
}

impl EthernetAdapters {
    /// Create a new [`EthernetAdapters`].
    pub fn new() -> Self {
        let mut adapters = Vec::new();
        unsafe /* ignore miri */ {
            let mut adapter = soem_bindings::ec_find_adapters();
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
            soem_bindings::ec_free_adapters(adapter);
            EthernetAdapters { adapters }
        }
    }
}

impl Default for EthernetAdapters {
    fn default() -> Self {
        Self::new()
    }
}
