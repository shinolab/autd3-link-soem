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
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender, bounded};
use time::ext::NumericalDuration;
use zerocopy::FromZeros;

use autd3_core::{
    geometry::Geometry,
    link::{RxMessage, TxMessage},
    sleep::Sleep,
};

use crate::local::error::SOEMError;

use super::{
    Context, State, Status, consts::*, iomap::IOMap, option::SOEMOption, smoothing::Smoothing,
    utils::is_autd3,
};

pub struct SOEMHandler {
    ctx: Arc<Context>,
    send_queue: Sender<Vec<TxMessage>>,
    buffer_queue: Receiver<Vec<TxMessage>>,
    is_open: Arc<AtomicBool>,
    io_map: Arc<Mutex<IOMap>>,
    ecat_th: Option<JoinHandle<Result<(), SOEMError>>>,
    ecat_check_th: Option<JoinHandle<()>>,
}

impl SOEMHandler {
    pub(crate) fn open_with_sleeper<
        F: Fn(usize, Status) + Send + Sync + 'static,
        S: Sleep + Send + 'static,
    >(
        err_handler: F,
        option: SOEMOption,
        geometry: &Geometry,
        sleeper: S,
    ) -> Result<Self, SOEMError> {
        tracing::debug!("Opening SOEM link: {:?}", option);

        option.validate()?;
        let ifname = option.ifname()?;

        let ctx = Arc::new(Context::new());

        tracing::info!("Initializing SOEM with interface {:?}.", ifname);
        ctx.init(ifname)?;

        let num_devices = if let Some(wc) = ctx.config_init() {
            if !geometry.is_empty() && wc != geometry.len() {
                return Err(SOEMError::SlaveNotFound(wc as _, geometry.len() as _));
            }
            wc
        } else {
            return Err(SOEMError::SlaveNotFound(0, geometry.len() as _));
        };
        tracing::info!(
            "Found {} slave{}.",
            num_devices,
            if num_devices > 1 { "s" } else { "" }
        );

        ctx.slaves().enumerate().try_for_each(|(i, slave)| {
            if is_autd3(slave) {
                Ok(())
            } else {
                tracing::error!("Slave[{}] is not an AUTD device.", i);
                Err(SOEMError::NoDeviceFound)
            }
        })?;

        tracing::info!(
            "Configuring Sync0 with cycle time {:?}.",
            option.sync0_cycle
        );
        ctx.configdc(option.sync0_cycle);
        ctx.set_po2so_config();

        wait_for_sync(
            &ctx,
            num_devices,
            option.sync_tolerance,
            option.sync_timeout,
        )?;

        let io_map = Arc::new(Mutex::new(IOMap::new(num_devices)));
        ctx.config_map_group(io_map.lock().unwrap().as_ptr() as _);

        tracing::info!("Checking if all devices are in safe operational state.");
        let reqstate = State::SAFE_OP;
        ctx.state_check(0, reqstate, EC_TIMEOUTSTATE * 3);
        let state = ctx.fetch_state(0);
        if state != reqstate {
            return Err(SOEMError::NotReachedRequiredState(reqstate, state));
        }
        tracing::info!("All devices are in safe operational state.");

        let is_open = Arc::new(AtomicBool::new(true));
        let wkc = Arc::new(AtomicI32::new(0));

        let state_check_interval = option.state_check_interval;
        let buf_size = option.buf_size.get();
        let (send_queue_sender, send_queue_receiver) = bounded(buf_size);
        let (buffer_queue_sender, buffer_queue_receiver) = bounded(buf_size);
        (0..buf_size).for_each(|_| {
            buffer_queue_sender
                .send(vec![TxMessage::new_zeroed(); num_devices])
                .unwrap()
        });
        let ecat_th = Some(std::thread::spawn({
            let is_open = is_open.clone();
            let io_map = io_map.clone();
            let wkc = wkc.clone();
            let ctx = ctx.clone();
            move || {
                ecat_run::<S>(
                    ctx,
                    is_open,
                    io_map,
                    wkc,
                    buffer_queue_sender,
                    send_queue_receiver,
                    sleeper,
                    option,
                )
            }
        }));

        tracing::info!("Setting all slaves to operational state.");
        ctx.set_state(0, State::OPERATIONAL);
        ctx.write_state(0);
        ctx.state_check(0, State::OPERATIONAL, EC_TIMEOUTSTATE);
        tracing::info!("All devices are in operational state.");
        let state = ctx.fetch_state(0);
        if state != State::OPERATIONAL {
            ctx.slaves().for_each(|slave| {
                tracing::error!(
                    "{} (State={}, StatusCode={:#04X})",
                    Context::alstatuscode2string(slave.ALstatuscode),
                    State::from(slave.state),
                    slave.ALstatuscode
                );
            });
            return Err(SOEMError::NotResponding);
        }

        tracing::info!(
            "Starting EtherCAT state check thread with interval {:?}.",
            state_check_interval
        );
        let ecat_check_th = Some(std::thread::spawn({
            let is_open = is_open.clone();
            let expected_wkc = ctx.expected_wkc();
            let ctx = ctx.clone();
            move || {
                while is_open.load(Ordering::Acquire) {
                    if wkc.load(Ordering::Relaxed) < expected_wkc || ctx.docheckstate() {
                        ctx.handle_error(&err_handler);
                    }
                    std::thread::sleep(state_check_interval);
                }
            }
        }));

        Ok(Self {
            ctx,
            send_queue: send_queue_sender,
            buffer_queue: buffer_queue_receiver,
            is_open,
            io_map,
            ecat_th,
            ecat_check_th,
        })
    }

    pub fn num_devices(&self) -> usize {
        self.ctx.num_devices()
    }

    pub fn close(&mut self) {
        if !self.is_open.load(Ordering::Acquire) {
            return;
        }
        self.is_open.store(false, Ordering::Release);

        while !self.send_queue.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
        }

        if let Some(handle) = self.ecat_th.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.ecat_check_th.take() {
            let _ = handle.join();
        }
        self.ctx.close();
    }

    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Acquire)
    }

    pub fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, crossbeam_channel::RecvError> {
        self.buffer_queue.recv()
    }

    pub fn send(
        &mut self,
        tx: Vec<TxMessage>,
    ) -> Result<(), crossbeam_channel::SendError<Vec<TxMessage>>> {
        self.send_queue.send(tx)
    }

    pub fn receive(
        &mut self,
        rx: &mut [RxMessage],
    ) -> Result<(), std::sync::PoisonError<std::sync::MutexGuard<'_, IOMap>>> {
        let io_map = self.io_map.lock()?;
        rx.copy_from_slice(io_map.input());
        Ok(())
    }

    pub fn clear_iomap(
        &mut self,
    ) -> Result<(), std::sync::PoisonError<std::sync::MutexGuard<'_, IOMap>>> {
        while !self.send_queue.is_empty() {
            std::thread::sleep(Duration::from_millis(100));
        }
        self.io_map.lock()?.clear();
        Ok(())
    }
}

fn wait_for_sync(
    ctx: &Context,
    num_devices: usize,
    tolerance: Duration,
    timeout: Duration,
) -> Result<(), SOEMError> {
    tracing::info!("Waiting for synchronization.");
    let max_diff = std::thread::scope(|s| {
        let (tx, rx) = bounded(1);
        let th = s.spawn(move || {
            let mut data = 0u64;
            loop {
                if rx.try_recv().is_ok() {
                    break;
                }
                ctx.frmw(
                    ECT_REG_DCSYSTIME,
                    std::mem::size_of::<u64>() as _,
                    &mut data as *mut _ as _,
                    EC_TIMEOUTRET,
                );
                std::thread::sleep(Duration::from_millis(1));
            }
        });

        std::thread::sleep(Duration::from_millis(100));

        let max_diff = if num_devices == 1 {
            Duration::ZERO
        } else {
            let mut last_diff = (0..num_devices - 1)
                .map(|_| tolerance.as_nanos() as u32)
                .collect::<Vec<_>>();
            let mut diff_averages = vec![Smoothing::new(0.2); num_devices - 1];
            let start = std::time::Instant::now();
            loop {
                let max_diff = ctx
                    .slaves()
                    .enumerate()
                    .skip(1)
                    .zip(last_diff.iter_mut())
                    .zip(diff_averages.iter_mut())
                    .fold(Duration::ZERO, |acc, (((i, slave), last_diff), ave)| {
                        let mut diff: u32 = 0;
                        let res = ctx.fprd(
                            slave,
                            ECT_REG_DCSYSDIFF as _,
                            std::mem::size_of::<u32>() as _,
                            &mut diff as *mut _ as *mut _,
                            EC_TIMEOUTRET as _,
                        );
                        let diff = if res != 1 {
                            tracing::trace!("Failed to read DCSYSDIFF[{}].", i);
                            *last_diff
                        } else {
                            *last_diff = diff;
                            diff
                        };
                        // DCSYSDIFF is not a 2's complement value.
                        // See RZ/T1 Group User's Manual: Hardware, 30.17.2.5
                        const MASK: u32 = 0x7fffffff;
                        let diff = if diff & (!MASK) != 0 {
                            -((diff & MASK) as i32)
                        } else {
                            diff as i32
                        };
                        let diff = Duration::from_nanos(ave.push(diff as _).abs() as _);
                        tracing::trace!("DCSYSDIFF[{}] = {:?}.", i, diff);
                        acc.max(diff)
                    });
                tracing::debug!("Maximum system time difference is {:?}.", max_diff);
                if max_diff < tolerance || start.elapsed() > timeout {
                    break max_diff;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        let _ = tx.send(());
        let _ = th.join();
        max_diff
    });

    if max_diff < tolerance {
        tracing::info!(
            "All devices are synchronized. Maximum system time difference is {:?}.",
            max_diff
        );
        Ok(())
    } else {
        Err(SOEMError::SynchronizeFailed(max_diff, tolerance))
    }
}

#[allow(clippy::too_many_arguments)]
fn ecat_run<S: Sleep>(
    ctx: Arc<Context>,
    is_open: Arc<AtomicBool>,
    io_map: Arc<Mutex<IOMap>>,
    wkc: Arc<AtomicI32>,
    buffer_queue_sender: Sender<Vec<TxMessage>>,
    receiver: Receiver<Vec<TxMessage>>,
    sleeper: S,
    option: SOEMOption,
) -> Result<(), SOEMError> {
    let cycle = option.send_cycle;
    tracing::info!("Starting EtherCAT thread with cycle time {:?}.", cycle);

    if let Some(affinity) = option.affinity {
        tracing::info!(
            "Setting CPU affinity for the EtherCAT thread to {:?}",
            affinity
        );
        if !core_affinity::set_for_current(affinity) {
            tracing::error!("Failed to set CPU affinity for the EtherCAT thread.");
            return Err(SOEMError::AffinitySetFailed(affinity));
        }
    }

    option.thread_priority.set_for_current()?;

    let mut cnt_miss_deadline = 0;
    let mut toff = time::Duration::ZERO;
    let mut timeerror = 0;
    let mut integral = 0;
    ctx.send_processdata();
    let mut ts = {
        let tp = time::OffsetDateTime::now_utc();
        let tp_unix_ns = tp.unix_timestamp_nanos();
        let cycle_ns = cycle.as_nanos() as i128;
        let ts_unix_ns = (tp_unix_ns / cycle_ns + 1) * cycle_ns;
        time::OffsetDateTime::from_unix_timestamp_nanos(ts_unix_ns).unwrap()
    };
    while is_open.load(Ordering::Acquire) {
        ts += cycle;
        ts += toff;

        let duration = ts - time::OffsetDateTime::now_utc();
        if duration > time::Duration::ZERO {
            sleeper.sleep(std::time::Duration::from_nanos(
                duration.whole_nanoseconds() as _,
            ));
            cnt_miss_deadline = 0;
        } else {
            cnt_miss_deadline += 1;
            if cnt_miss_deadline == 1000 {
                tracing::warn!(
                    "Slow network was detected. Increase send_cycle and sync0_cycle and restart the program, or reboot the network adapter and device."
                );
                cnt_miss_deadline = 0;
            }
        }

        wkc.store(
            ctx.receive_processdata(EC_TIMEOUTRET as i32),
            Ordering::Relaxed,
        );

        toff = ec_sync(
            ctx.dctime(),
            cycle.as_nanos() as _,
            &mut timeerror,
            &mut integral,
        );

        if let Ok(tx) = receiver.try_recv() {
            match io_map.lock() {
                Ok(mut io_map) => io_map.copy_from(&tx),
                Err(_) => {
                    is_open.store(false, Ordering::Release);
                    break;
                }
            }
            let _ = buffer_queue_sender.send(tx);
        }
        ctx.send_processdata();
    }
    Ok(())
}

fn ec_sync(
    reftime: i64,
    cycletime: i64,
    timeerror: &mut i64,
    integral: &mut i64,
) -> time::Duration {
    const KP: f32 = 0.01;
    const KI: f32 = 0.00002;
    let mut delta = (reftime - 500000) % cycletime;
    if delta > (cycletime / 2) {
        delta -= cycletime;
    }
    *timeerror = -delta;
    *integral += *timeerror;
    (((KP * *timeerror as f32) + (KI * *integral as f32)) as i64).nanoseconds()
}

impl Drop for SOEMHandler {
    fn drop(&mut self) {
        self.close();
    }
}
