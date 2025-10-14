// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use spin_sleep::SpinSleeper as _SpinSleeper;

use autd3_core::{
    geometry::Geometry,
    link::{Link, LinkError, RxMessage, TxMessage},
    sleep::Sleeper,
};

use crate::inner::{SOEMHandler, SOEMOptionFull};

use super::Status;

#[derive(Default)]
pub struct SpinSleeper {
    inner: _SpinSleeper,
}

impl autd3_core::sleep::Sleeper for SpinSleeper {
    fn sleep(&self, dur: std::time::Duration) {
        self.inner.sleep(dur)
    }
}

/// A [`Link`] using [SOEM].
///
/// [SOEM]: https://github.com/OpenEtherCATsociety/SOEM
pub struct SOEM<F: Fn(u16, Status) + Send + Sync + 'static, S: Sleeper> {
    option: Option<(F, SOEMOptionFull, S)>,
    handler: Option<SOEMHandler>,
}

impl<F: Fn(u16, Status) + Send + Sync + 'static> SOEM<F, SpinSleeper> {
    /// Creates a new [`SOEM`].
    pub fn new(err_handler: F, option: impl Into<SOEMOptionFull>) -> SOEM<F, SpinSleeper> {
        SOEM::with_sleeper(err_handler, option, SpinSleeper::default())
    }
}

impl<F: Fn(u16, Status) + Send + Sync + 'static, S: Sleeper> SOEM<F, S> {
    /// Creates a new [`SOEM`] with a sleeper
    pub fn with_sleeper(
        err_handler: F,
        option: impl Into<SOEMOptionFull>,
        sleeper: S,
    ) -> SOEM<F, S> {
        SOEM {
            option: Some((err_handler, option.into(), sleeper)),
            handler: None,
        }
    }
}

impl<F: Fn(u16, Status) + Send + Sync + 'static, S: Sleeper + Send + 'static> Link for SOEM<F, S> {
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
