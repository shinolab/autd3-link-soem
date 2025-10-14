// Copyright (c) 2022-2025 Shun Suzuki
//
// This file is part of autd3-link-soem.
//
// autd3-link-soem is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License.
//
// autd3-link-soem is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Foobar. If not, see <https://www.gnu.org/licenses/>.

use std::{
    cell::Cell,
    ffi::{CString, c_void},
    sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    },
    time::Duration,
};

use crate::error::SOEMError;

use super::*;

#[derive(Clone)]
pub struct Context {
    ctx: ecx_contextt,
    initialized: Cell<bool>,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Context {
    pub fn new() -> Self {
        Self {
            ctx: unsafe { std::mem::zeroed() },
            initialized: Cell::new(false),
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn ctx_mut(&self) -> &mut ecx_contextt {
        unsafe { self.as_mut_ptr().as_mut().unwrap() }
    }

    pub fn init(&self, ifname: CString) -> Result<(), SOEMError> {
        if unsafe { ecx_init(self.as_mut_ptr(), ifname.as_ptr()) } > 0 {
            self.initialized.set(true);
            Ok(())
        } else {
            Err(SOEMError::NoSocketConnection(ifname))
        }
    }

    pub fn config_init(&self) -> Option<usize> {
        let wc = unsafe { ecx_config_init(self.as_mut_ptr()) };
        if wc <= 0 { None } else { Some(wc as _) }
    }

    pub fn config_map_group(&self, ptr: *mut c_void) {
        unsafe { ecx_config_map_group(self.as_mut_ptr(), ptr, 0) };
    }

    pub fn configdc(&self, userdata: Duration) {
        self.ctx_mut().userdata = Box::into_raw(Box::new(userdata)) as *mut _;
        unsafe { ecx_configdc(self.as_mut_ptr()) };
    }

    pub fn set_po2so_config(&self) {
        self.slaves_mut()
            .for_each(|slave| slave.PO2SOconfig = Some(po2so_config));
    }

    pub fn docheckstate(&self) -> bool {
        self.ctx_mut().grouplist[0].docheckstate != 0
    }

    fn reconfig_slave(&self, slave: u16, timeout: u32) -> i32 {
        unsafe { ecx_reconfig_slave(self.as_mut_ptr(), slave, timeout as _) }
    }

    fn recover_slave(&self, slave: u16, timeout: u32) -> i32 {
        unsafe { ecx_recover_slave(self.as_mut_ptr(), slave, timeout as _) }
    }

    pub fn state_check(&self, slave: u16, reqstate: State, timeout: u32) {
        unsafe { ecx_statecheck(self.as_mut_ptr(), slave, reqstate.state(), timeout as _) };
    }

    pub fn fetch_state(&self, idx: usize) -> State {
        State::from(self.ctx.slavelist[idx].state)
    }

    pub fn set_state(&self, idx: usize, state: State) {
        self.ctx_mut().slavelist[idx].state = state.state();
    }

    pub fn read_state(&self) {
        unsafe { ecx_readstate(self.as_mut_ptr()) };
    }

    pub fn write_state(&self, slave: u16) {
        unsafe { ecx_writestate(self.as_mut_ptr(), slave as _) };
    }

    pub fn frmw(&self, ado: u16, length: u16, data: *mut c_void, timeout: u32) -> i32 {
        unsafe {
            ecx_FRMW(
                self.port(),
                self.slave(0).configadr,
                ado,
                length,
                data,
                timeout as _,
            )
        }
    }

    pub fn fprd(
        &self,
        slave: &ec_slavet,
        ado: u16,
        length: u16,
        data: *mut c_void,
        timeout: u32,
    ) -> i32 {
        unsafe {
            ecx_FPRD(
                self.port(),
                slave.configadr,
                ado,
                length,
                data,
                timeout as _,
            )
        }
    }

    pub fn send_processdata(&self) {
        unsafe { ecx_send_processdata(self.as_mut_ptr()) };
    }

    pub fn receive_processdata(&self, timeout: i32) -> i32 {
        unsafe { ecx_receive_processdata(self.as_mut_ptr(), timeout as _) }
    }

    pub fn dctime(&self) -> i64 {
        self.ctx.DCtime
    }

    pub fn expected_wkc(&self) -> i32 {
        (self.ctx.grouplist[0].outputsWKC * 2 + self.ctx.grouplist[0].inputsWKC) as i32
    }

    pub fn slave(&self, idx: usize) -> &ec_slavet {
        &self.ctx.slavelist[idx + 1]
    }

    pub fn slaves(&self) -> impl std::iter::Iterator<Item = &ec_slavet> {
        let count = self.ctx.slavecount as usize;
        self.ctx.slavelist.iter().skip(1).take(count)
    }

    pub fn slaves_mut(&self) -> impl std::iter::Iterator<Item = &mut ec_slavet> {
        let count = self.ctx.slavecount as usize;
        self.ctx_mut().slavelist.iter_mut().skip(1).take(count)
    }

    pub fn port(&self) -> *mut ecx_portt {
        &raw const self.ctx.port as _
    }

    pub fn as_mut_ptr(&self) -> *mut ecx_contextt {
        &self.ctx as *const _ as _
    }

    pub fn close(&self) {
        if self.initialized.replace(false) {
            if !self.ctx.userdata.is_null() {
                let cyc_time = unsafe { Box::from_raw(self.ctx.userdata as *mut Duration) };
                self.ctx_mut().userdata = std::ptr::null_mut();
                let cyc_time = cyc_time.as_nanos() as _;

                (1..=self.ctx.slavecount as u16).for_each(|i| {
                    unsafe { ecx_dcsync0(self.as_mut_ptr(), i, 0, cyc_time, 0) };
                });
            }

            self.set_state(0, State::INIT);
            self.write_state(0);

            unsafe {
                ecx_close(self.as_mut_ptr());
            }
        }
    }
}

impl Context {
    pub fn handle_error<F: Fn(u16, Status)>(&self, handler: &F, do_wkc_check: &Arc<AtomicI32>) {
        self.ctx_mut().grouplist[0].docheckstate = 0;
        self.read_state();
        self.slaves_mut().enumerate().for_each(|(i, slave)| {
            let slave_idx = (i + 1) as u16;
            let state = State::from(slave.state);
            if state != State::OPERATIONAL {
                self.ctx_mut().grouplist[0].docheckstate = 1;
                if state.is_safe_op() && state.is_error() {
                    (handler)(slave_idx, Status::Error);
                    slave.state = ec_state_EC_STATE_SAFE_OP as u16 + ec_state_EC_STATE_ACK as u16;
                    self.write_state(slave_idx);
                } else if state.is_safe_op() {
                    (handler)(slave_idx, Status::StateChanged);
                    slave.state = ec_state_EC_STATE_OPERATIONAL as _;
                    self.write_state(slave_idx);
                } else if state.is_some() {
                    if self.reconfig_slave(slave_idx, 500) >= ec_state_EC_STATE_PRE_OP as _ {
                        slave.islost = 0;
                    }
                } else if slave.islost == 0 {
                    self.state_check(slave_idx, State::OPERATIONAL, EC_TIMEOUTRET);
                    if state.is_none() {
                        slave.islost = 1;
                        unsafe { std::ptr::write_bytes(slave.inputs, 0x00, slave.Ibytes as _) };
                        (handler)(slave_idx, Status::Lost);
                    }
                }
            }
            if slave.islost != 0 {
                if state.is_none() {
                    if self.recover_slave(slave_idx, 500) != 0 {
                        slave.islost = 0;
                        (handler)(slave_idx, Status::Recovered);
                    }
                } else {
                    slave.islost = 0;
                }
            }
        });

        if self.ctx.grouplist[0].docheckstate == 0 {
            (handler)(0, Status::Resumed);
        }
        do_wkc_check.store(0, Ordering::Relaxed);
    }
}

impl Context {
    pub fn alstatuscode2string(code: u16) -> String {
        unsafe { std::ffi::CStr::from_ptr(ec_ALstatuscode2string(code)) }
            .to_string_lossy()
            .into_owned()
    }
}

unsafe extern "C" fn po2so_config(context: *mut ecx_contextt, slave: u16) -> i32 {
    unsafe {
        let cyc_time = ((*context).userdata as *mut Duration)
            .as_ref()
            .unwrap()
            .as_nanos() as _;
        ecx_dcsync0(context, slave, 1, cyc_time, 0);
    }
    0
}

impl Drop for Context {
    fn drop(&mut self) {
        self.close();
    }
}
