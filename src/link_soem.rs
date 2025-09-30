// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use spin_sleep::SpinSleeper;

use autd3_core::{
    geometry::Geometry,
    link::{Link, LinkError, RxMessage, TxMessage},
    sleep::Sleep,
};

use crate::inner::{SOEMHandler, SOEMOption};

use super::Status;

/// A [`Link`] using [SOEM].
///
/// [SOEM]: https://github.com/OpenEtherCATsociety/SOEM
pub struct SOEM<F: Fn(u16, Status) + Send + Sync + 'static, S: Sleep> {
    option: Option<(F, SOEMOption, S)>,
    handler: Option<SOEMHandler>,
}

impl<F: Fn(u16, Status) + Send + Sync + 'static> SOEM<F, SpinSleeper> {
    /// Creates a new [`SOEM`].
    pub fn new(err_handler: F, option: SOEMOption) -> SOEM<F, SpinSleeper> {
        SOEM::with_sleeper(err_handler, option, SpinSleeper::default())
    }
}

impl<F: Fn(u16, Status) + Send + Sync + 'static, S: Sleep> SOEM<F, S> {
    /// Creates a new [`SOEM`] with a sleeper
    pub fn with_sleeper(err_handler: F, option: SOEMOption, sleeper: S) -> SOEM<F, S> {
        SOEM {
            option: Some((err_handler, option, sleeper)),
            handler: None,
        }
    }

    #[doc(hidden)]
    pub fn num_devices(&self) -> usize {
        self.handler.as_ref().map_or(0, |inner| inner.num_devices())
    }

    #[doc(hidden)]
    pub fn clear_iomap(&mut self) -> Result<(), LinkError> {
        self.handler.as_mut().map_or(Ok(()), |inner| {
            inner.clear_iomap().map_err(|_| LinkError::closed())
        })
    }
}

impl<F: Fn(u16, Status) + Send + Sync + 'static, S: Sleep + Send + 'static> Link for SOEM<F, S> {
    fn open(&mut self, geometry: &Geometry) -> Result<(), LinkError> {
        if let Some((err_handler, option, sleeper)) = self.option.take() {
            self.handler = Some(SOEMHandler::open_with_sleeper(
                err_handler,
                option,
                geometry,
                sleeper,
            )?);
        }
        Ok(())
    }

    fn close(&mut self) -> Result<(), LinkError> {
        self.handler.take().map_or(Ok(()), |mut handler| {
            handler.close();
            Ok(())
        })
    }

    fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, LinkError> {
        self.handler
            .as_mut()
            .map_or(Err(LinkError::new("Link is closed")), |inner| {
                inner.alloc_tx_buffer().map_err(|_| LinkError::closed())
            })
    }

    fn send(&mut self, tx: Vec<TxMessage>) -> Result<(), LinkError> {
        self.handler
            .as_mut()
            .map_or(Err(LinkError::new("Link is closed")), |inner| {
                inner.send(tx).map_err(|_| LinkError::closed())
            })
    }

    fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        self.handler
            .as_mut()
            .map_or(Err(LinkError::new("Link is closed")), |inner| {
                inner.receive(rx).map_err(|_| LinkError::closed())
            })
    }

    fn is_open(&self) -> bool {
        self.handler.as_ref().is_some_and(|inner| inner.is_open())
    }
}

#[cfg(feature = "async")]
use autd3_core::link::AsyncLink;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
impl<F: Fn(u16, Status) + Send + Sync + 'static, S: Sleep + Send + 'static> AsyncLink
    for SOEM<F, S>
{
    async fn open(&mut self, geometry: &Geometry) -> Result<(), LinkError> {
        <Self as Link>::open(self, geometry)
    }

    async fn close(&mut self) -> Result<(), LinkError> {
        <Self as Link>::close(self)
    }

    async fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, LinkError> {
        <Self as Link>::alloc_tx_buffer(self)
    }

    async fn send(&mut self, tx: Vec<TxMessage>) -> Result<(), LinkError> {
        <Self as Link>::send(self, tx)
    }

    async fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        <Self as Link>::receive(self, rx)
    }

    fn is_open(&self) -> bool {
        <Self as Link>::is_open(self)
    }
}
