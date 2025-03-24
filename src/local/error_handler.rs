#![allow(static_mut_refs)]

// TODO: static mut will be deprecated in Rust 2024 edition?

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicI32, Ordering},
};

use crate::local::soem_bindings::*;

use derive_more::Display;

#[derive(Debug, Clone, PartialEq, Display)]
#[repr(u8)]
/// The status of the EtherCAT slave.
pub enum Status {
    /// The slave is in SAFE_OP + ERROR.
    #[display("slave is in SAFE_OP + ERROR, attempting ack")]
    Error = 0,
    /// The slave is lost.
    #[display("slave is lost")]
    Lost = 1,
    /// The slave is in SAFE_OP.
    #[display("slave is in SAFE_OP, change to OPERATIONAL")]
    StateChanged = 2,
}

pub struct EcatErrorHandler<F: Fn(usize, Status)> {
    pub(crate) err_handler: F,
}

impl<F: Fn(usize, Status)> EcatErrorHandler<F> {
    pub(crate) fn run(
        &self,
        is_open: Arc<AtomicBool>,
        wkc: Arc<AtomicI32>,
        expected_wkc: i32,
        state_check_interval: std::time::Duration,
    ) {
        unsafe /* ignore miri */ {
            while is_open.load(Ordering::Acquire) {
                if wkc.load(Ordering::Relaxed) < expected_wkc || ec_group[0].docheckstate != 0 {
                    self.handle();
                }
                std::thread::sleep(state_check_interval);
            }
        }
    }

    fn handle(&self) -> bool {
        unsafe /* ignore miri */ {
            ec_group[0].docheckstate = 0;
            ec_readstate();
            ec_slave
                .iter_mut()
                .enumerate()
                .skip(1)
                .take(ec_slavecount as usize)
                .for_each(|(i, slave)| {
                    if slave.state != ec_state_EC_STATE_OPERATIONAL as u16 {
                        ec_group[0].docheckstate = 1;
                        if slave.state
                            == ec_state_EC_STATE_SAFE_OP as u16 + ec_state_EC_STATE_ERROR as u16
                        {
                            (self.err_handler)(i - 1, Status::Error);
                            slave.state =
                                ec_state_EC_STATE_SAFE_OP as u16 + ec_state_EC_STATE_ACK as u16;
                            ec_writestate(i as _);
                        } else if slave.state == ec_state_EC_STATE_SAFE_OP as u16 {
                            (self.err_handler)(i - 1, Status::StateChanged);
                            slave.state = ec_state_EC_STATE_OPERATIONAL as _;
                            ec_writestate(i as _);
                        } else if slave.state > ec_state_EC_STATE_NONE as u16 {
                            if ec_reconfig_slave(i as _, 500) != 0 {
                                slave.islost = 0;
                            }
                        } else if slave.islost == 0 {
                            ec_statecheck(
                                i as _,
                                ec_state_EC_STATE_OPERATIONAL as _,
                                EC_TIMEOUTRET as _,
                            );
                            if slave.state == ec_state_EC_STATE_NONE as u16 {
                                slave.islost = 1;
                                (self.err_handler)(i - 1, Status::Lost);
                            }
                        }
                    }
                    if slave.islost != 0 {
                        if slave.state == ec_state_EC_STATE_NONE as u16 {
                            if ec_recover_slave(i as _, 500) != 0 {
                                slave.islost = 0;
                            }
                        } else {
                            slave.islost = 0;
                        }
                    }
                });

            if ec_group[0].docheckstate == 0 {
                return true;
            }

            ec_slave
                .iter()
                .skip(1)
                .take(ec_slavecount as usize)
                .all(|slave| slave.islost == 0)
        }
    }
}
