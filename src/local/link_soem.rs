use std::{
    ffi::{CString, c_void},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
    thread::JoinHandle,
    time::Duration,
};

use crossbeam_channel::{Receiver, Sender, bounded};
use spin_sleep::SpinSleeper;
use ta::{Next, indicators::ExponentialMovingAverage};
use thread_priority::ThreadPriority;
use time::ext::NumericalDuration;
use zerocopy::FromZeros;

use autd3_core::{
    ethercat::EC_CYCLE_TIME_BASE,
    geometry::Geometry,
    link::{Link, LinkError, RxMessage, TxMessage},
    sleep::Sleep,
};

use super::{
    Status, error::SOEMError, error_handler::EcatErrorHandler, ethernet_adapters::EthernetAdapters,
    iomap::IOMap, option::SOEMOption, soem_bindings::*, state::EcStatus,
};

struct SOEMInner {
    send_queue: Sender<Vec<TxMessage>>,
    buffer_queue: Receiver<Vec<TxMessage>>,
    is_open: Arc<AtomicBool>,
    send_cycle: Duration,
    io_map: Arc<Mutex<IOMap>>,
    init_guard: Option<SOEMInitGuard>,
    config_dc_guard: Option<SOEMDCConfigGuard>,
    op_state_guard: Option<OpStateGuard>,
    ecat_th_guard: Option<SOEMECatThreadGuard>,
    ecat_check_th_guard: Option<SOEMEcatCheckThreadGuard>,
}

impl SOEMInner {
    pub(crate) fn open_with_sleeper<
        F: Fn(usize, Status) + Send + Sync + 'static,
        S: Sleep + Send + 'static,
    >(
        err_handler: F,
        option: SOEMOption,
        geometry: &Geometry,
        sleeper: S,
    ) -> Result<Self, LinkError> {
        tracing::debug!("Opening SOEM link: {:?}", option);

        unsafe {
            let SOEMOption {
                buf_size,
                ifname,
                state_check_interval,
                sync0_cycle,
                send_cycle,
                thread_priority,
                #[cfg(target_os = "windows")]
                process_priority,
                sync_tolerance,
                sync_timeout,
            } = option;

            if sync0_cycle.is_zero() || sync0_cycle.as_nanos() % EC_CYCLE_TIME_BASE.as_nanos() != 0
            {
                return Err(SOEMError::InvalidCycle(sync0_cycle).into());
            }
            if send_cycle.is_zero() || send_cycle.as_nanos() % EC_CYCLE_TIME_BASE.as_nanos() != 0 {
                return Err(SOEMError::InvalidCycle(send_cycle).into());
            }

            let ifname = if ifname.is_empty() {
                tracing::info!("No interface name is specified. Looking up AUTD device.");
                let ifname = Self::lookup_autd()?;
                tracing::info!("Found AUTD device on {}.", ifname);
                ifname
            } else {
                ifname.clone()
            };

            tracing::info!("Initializing SOEM with interface {}.", ifname);
            let init_guard = SOEMInitGuard::new(ifname)?;

            let wc = ec_config_init(0);
            tracing::info!("Found {} slaves.", wc);
            if wc <= 0 || (!geometry.is_empty() && wc as usize != geometry.len()) {
                return Err(SOEMError::SlaveNotFound(wc as _, geometry.len() as _).into());
            }
            (1..=wc).try_for_each(|i| {
                if Self::is_autd3(i) {
                    Ok(())
                } else {
                    tracing::error!("Slave[{}] is not an AUTD device.", i - 1);
                    Err(SOEMError::NoDeviceFound)
                }
            })?;
            let num_devices = wc as _;

            let (send_queue_sender, send_queue_receiver) = bounded(buf_size.get());
            let (buffer_queue_sender, buffer_queue_receiver) = bounded(buf_size.get());
            (0..buf_size.get()).for_each(|_| {
                buffer_queue_sender
                    .send(vec![TxMessage::new_zeroed(); num_devices])
                    .unwrap()
            });

            let is_open = Arc::new(AtomicBool::new(true));
            let io_map = Arc::new(Mutex::new(IOMap::new(num_devices)));
            let config_dc_guard = SOEMDCConfigGuard::new(sync0_cycle);

            tracing::info!("Configuring Sync0 with cycle time {:?}.", sync0_cycle);
            config_dc_guard.set_dc_config();

            tracing::info!("Waiting for synchronization.");
            let (tx, rx) = bounded(1);
            let th = std::thread::spawn(move || {
                let mut data = 0u64;
                loop {
                    if rx.try_recv().is_ok() {
                        break;
                    }
                    ec_FRMW(
                        ec_slave[1].configadr,
                        ECT_REG_DCSYSTIME as _,
                        std::mem::size_of::<u64>() as _,
                        &mut data as *mut _ as *mut _,
                        EC_TIMEOUTRET as _,
                    );
                    std::thread::sleep(Duration::from_millis(1));
                }
            });
            std::thread::sleep(Duration::from_millis(100));
            let max_diff = if wc == 1 {
                Duration::ZERO
            } else {
                let mut last_diff = (0..wc as usize - 1)
                    .map(|_| sync_tolerance.as_nanos() as u32)
                    .collect::<Vec<_>>();
                let mut diff_averages =
                    vec![ExponentialMovingAverage::new(9).unwrap(); (wc - 1) as usize];
                let start = std::time::Instant::now();
                loop {
                    let max_diff = (2..=wc)
                        .zip(last_diff.iter_mut())
                        .zip(diff_averages.iter_mut())
                        .fold(Duration::ZERO, |acc, ((slave, last_diff), ave)| {
                            let mut diff: u32 = 0;
                            let res = ec_FPRD(
                                ec_slave[slave as usize].configadr,
                                ECT_REG_DCSYSDIFF as _,
                                std::mem::size_of::<u32>() as _,
                                &mut diff as *mut _ as *mut _,
                                EC_TIMEOUTRET as _,
                            );
                            let diff = if res != 1 {
                                tracing::trace!("Failed to read DCSYSDIFF[{}].", slave - 1);
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
                            let diff = Duration::from_nanos(ave.next(diff as f64).abs() as _);
                            tracing::trace!("DCSYSDIFF[{}] = {:?}.", slave - 1, diff);
                            acc.max(diff)
                        });
                    tracing::debug!("Maximum system time difference is {:?}.", max_diff);
                    if max_diff < sync_tolerance || start.elapsed() > sync_timeout {
                        break max_diff;
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
            };
            let _ = tx.send(());
            let _ = th.join();
            if max_diff < sync_tolerance {
                tracing::info!(
                    "All devices are synchronized. Maximum system time difference is {:?}.",
                    max_diff
                );
            } else {
                return Err(SOEMError::SynchronizeFailed(max_diff, sync_tolerance).into());
            }

            let mut result = Self {
                send_queue: send_queue_sender,
                buffer_queue: buffer_queue_receiver,
                is_open,
                send_cycle,
                io_map,
                init_guard: Some(init_guard),
                config_dc_guard: Some(config_dc_guard),
                op_state_guard: None,
                ecat_th_guard: None,
                ecat_check_th_guard: None,
            };

            ec_config_map(result.io_map.lock().unwrap().as_ptr() as *mut c_void);

            result.op_state_guard = Some(OpStateGuard {});

            tracing::info!("Checking if all devices are in safe operational state.");
            OpStateGuard::to_safe_op(num_devices)?;

            tracing::info!(
                "All devices are in safe operational state. Switching to operational state."
            );
            OpStateGuard::to_op();
            tracing::info!("All devices are in operational state.");

            let wkc = Arc::new(AtomicI32::new(0));
            tracing::info!("Starting EtherCAT thread with cycle time {:?}.", send_cycle);
            result.ecat_th_guard = Some(SOEMECatThreadGuard::new(
                result.is_open.clone(),
                wkc.clone(),
                result.io_map.clone(),
                buffer_queue_sender,
                send_queue_receiver,
                sleeper,
                thread_priority,
                #[cfg(target_os = "windows")]
                process_priority,
                send_cycle,
            ));

            if !OpStateGuard::is_op_state() {
                return Err(SOEMError::NotResponding(EcStatus::new(num_devices)).into());
            }

            tracing::info!(
                "Starting EtherCAT state check thread with interval {:?}.",
                state_check_interval
            );
            result.ecat_check_th_guard = Some(SOEMEcatCheckThreadGuard::new(
                result.is_open.clone(),
                err_handler,
                wkc.clone(),
                state_check_interval,
            ));

            Ok(result)
        }
    }

    fn close(&mut self) -> Result<(), LinkError> {
        if !self.is_open.load(Ordering::Acquire) {
            return Ok(());
        }
        self.is_open.store(false, Ordering::Release);

        while !self.send_queue.is_empty() {
            std::thread::sleep(self.send_cycle);
        }

        let _ = self.ecat_th_guard.take();
        let _ = self.ecat_check_th_guard.take();
        let _ = self.config_dc_guard.take();
        let _ = self.op_state_guard.take();
        let _ = self.init_guard.take();

        Ok(())
    }

    fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, LinkError> {
        self.buffer_queue.recv().map_err(|_| LinkError::closed())
    }

    fn send(&mut self, tx: Vec<TxMessage>) -> Result<(), LinkError> {
        self.send_queue.send(tx).map_err(|_| LinkError::closed())?;
        Ok(())
    }

    fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        let io_map = self.io_map.lock().map_err(|_| LinkError::closed())?;
        rx.copy_from_slice(io_map.input());
        Ok(())
    }

    fn is_autd3(i: i32) -> bool {
        unsafe {
            String::from_utf8(
                ec_slave[i as usize]
                    .name
                    .into_iter()
                    .take_while(|&c| c != 0)
                    .map(|c| c as u8)
                    .collect(),
            )
            .map(|name| {
                tracing::trace!("Slave[{}] name: {}", i - 1, name);
                name == "AUTD"
            })
            .unwrap_or(false)
        }
    }

    fn lookup_autd() -> Result<String, SOEMError> {
        let adapters = EthernetAdapters::new();

        tracing::debug!("Found {} network adapters.", adapters.len());

        adapters
            .into_iter()
            .find(|adapter| unsafe {
                let ifname = match std::ffi::CString::new(adapter.name().to_owned()) {
                    Ok(ifname) => ifname,
                    Err(_) => return false,
                };
                tracing::debug!("Searching AUTD device on {}.", adapter.name());
                if ec_init(ifname.as_ptr()) <= 0 {
                    tracing::trace!("Failed to initialize SOEM on {}.", adapter.name());
                    ec_close();
                    return false;
                }
                let wc = ec_config_init(0);
                if wc <= 0 {
                    tracing::trace!("No slave found on {}.", adapter.name());
                    ec_close();
                    return false;
                }
                tracing::trace!("Found {} slaves on {}.", wc, adapter.name());
                let found = (1..=wc).all(Self::is_autd3);
                ec_close();
                found
            })
            .map_or_else(
                || Err(SOEMError::NoDeviceFound),
                |adapter| Ok(adapter.name().to_owned()),
            )
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

impl Drop for SOEMInner {
    fn drop(&mut self) {
        self.is_open.store(false, Ordering::Release);
        let _ = self.ecat_th_guard.take();
        let _ = self.ecat_check_th_guard.take();
        let _ = self.config_dc_guard.take();
        let _ = self.op_state_guard.take();
        let _ = self.init_guard.take();
    }
}

/// A [`Link`] using [SOEM].
///
/// [SOEM]: https://github.com/OpenEtherCATsociety/SOEM
pub struct SOEM<F: Fn(usize, Status) + Send + Sync + 'static, S: Sleep> {
    option: Option<(F, SOEMOption, S)>,
    inner: Option<SOEMInner>,
}

impl<F: Fn(usize, Status) + Send + Sync + 'static> SOEM<F, SpinSleeper> {
    /// Creates a new [`SOEM`].
    pub fn new(err_handler: F, option: SOEMOption) -> SOEM<F, SpinSleeper> {
        SOEM::with_sleeper(err_handler, option, SpinSleeper::default())
    }
}

impl<F: Fn(usize, Status) + Send + Sync + 'static, S: Sleep> SOEM<F, S> {
    /// Creates a new [`SOEM`] with a sleeper
    pub fn with_sleeper(err_handler: F, option: SOEMOption, sleeper: S) -> SOEM<F, S> {
        SOEM {
            option: Some((err_handler, option, sleeper)),
            inner: None,
        }
    }

    #[doc(hidden)]
    pub fn num_devices(&self) -> usize {
        unsafe { ec_slavecount as usize }
    }

    #[doc(hidden)]
    pub fn clear_iomap(
        &mut self,
    ) -> Result<(), std::sync::PoisonError<std::sync::MutexGuard<'_, IOMap>>> {
        self.inner
            .as_mut()
            .map_or(Ok(()), |inner| inner.clear_iomap())
    }
}

impl<F: Fn(usize, Status) + Send + Sync + 'static, S: Sleep + Send + 'static> Link for SOEM<F, S> {
    fn open(&mut self, geometry: &Geometry) -> Result<(), LinkError> {
        if let Some((err_handler, option, sleeper)) = self.option.take() {
            self.inner = Some(SOEMInner::open_with_sleeper(
                err_handler,
                option,
                geometry,
                sleeper,
            )?);
        }
        Ok(())
    }

    fn close(&mut self) -> Result<(), LinkError> {
        self.inner.take().map_or(Ok(()), |mut inner| inner.close())
    }

    fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, LinkError> {
        self.inner
            .as_mut()
            .map_or(Err(LinkError::new("Link is closed")), |inner| {
                inner.alloc_tx_buffer()
            })
    }

    fn send(&mut self, tx: Vec<TxMessage>) -> Result<(), LinkError> {
        self.inner
            .as_mut()
            .map_or(Err(LinkError::new("Link is closed")), |inner| {
                inner.send(tx)
            })
    }

    fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        self.inner
            .as_mut()
            .map_or(Err(LinkError::new("Link is closed")), |inner| {
                inner.receive(rx)
            })
    }

    fn is_open(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.is_open.load(Ordering::Acquire))
    }
}

struct SOEMInitGuard;

impl SOEMInitGuard {
    fn new(ifname: String) -> Result<Self, SOEMError> {
        let ifname_c = CString::new(ifname.as_str())
            .map_err(|_| SOEMError::InvalidInterfaceName(ifname.clone()))?;
        if unsafe { ec_init(ifname_c.as_ptr()) <= 0 } {
            return Err(SOEMError::NoSocketConnection(ifname));
        }
        Ok(Self)
    }
}

impl Drop for SOEMInitGuard {
    fn drop(&mut self) {
        unsafe {
            ec_close();
        }
    }
}

struct SOEMDCConfigGuard {}

unsafe extern "C" fn po2so_config(context: *mut ecx_contextt, slave: uint16) -> i32 {
    unsafe {
        let cyc_time = ((*context).userdata as *mut Duration)
            .as_ref()
            .unwrap()
            .as_nanos() as _;
        ec_dcsync0(slave, 1, cyc_time, 0);
    }
    0
}

impl SOEMDCConfigGuard {
    fn new(ec_sync0_cycle: Duration) -> Self {
        unsafe {
            ecx_context.userdata = Box::into_raw(Box::new(ec_sync0_cycle)) as *mut _;
            ec_configdc();
        }
        Self {}
    }

    fn set_dc_config(&self) {
        unsafe {
            (1..=ec_slavecount as usize).for_each(|i| {
                ec_slave[i].PO2SOconfigx = Some(po2so_config);
            });
        }
    }
}

impl Drop for SOEMDCConfigGuard {
    fn drop(&mut self) {
        unsafe {
            if ecx_context.userdata.is_null() {
                return;
            }

            let cyc_time = Box::from_raw(ecx_context.userdata as *mut Duration);
            let cyc_time = cyc_time.as_nanos() as _;
            (1..=ec_slavecount as u16).for_each(|i| {
                ec_dcsync0(i, 0, cyc_time, 0);
            });
        }
    }
}

struct OpStateGuard;

impl OpStateGuard {
    fn to_safe_op(num_devices: usize) -> Result<(), SOEMError> {
        unsafe {
            ec_statecheck(0, ec_state_EC_STATE_SAFE_OP as u16, EC_TIMEOUTSTATE as i32);
            if ec_slave[0].state != ec_state_EC_STATE_SAFE_OP as u16 {
                return Err(SOEMError::NotReachedSafeOp(ec_slave[0].state));
            }
            ec_readstate();
            if ec_slave[0].state != ec_state_EC_STATE_SAFE_OP as u16 {
                return Err(SOEMError::NotResponding(EcStatus::new(num_devices)));
            }
        }

        Ok(())
    }

    fn to_op() {
        unsafe {
            ec_slave[0].state = ec_state_EC_STATE_OPERATIONAL as u16;
            ec_writestate(0);
        }
    }

    fn is_op_state() -> bool {
        unsafe {
            ec_statecheck(
                0,
                ec_state_EC_STATE_OPERATIONAL as u16,
                5 * EC_TIMEOUTSTATE as i32,
            );
            ec_slave[0].state == ec_state_EC_STATE_OPERATIONAL as u16
        }
    }
}

impl Drop for OpStateGuard {
    fn drop(&mut self) {
        unsafe {
            ec_slave[0].state = ec_state_EC_STATE_INIT as u16;
            ec_writestate(0);
        }
    }
}

struct SOEMECatThreadGuard {
    ecatth_handle: Option<JoinHandle<Result<(), SOEMError>>>,
}

impl SOEMECatThreadGuard {
    #[allow(clippy::too_many_arguments)]
    fn new<S: Sleep + Send + 'static>(
        is_open: Arc<AtomicBool>,
        wkc: Arc<AtomicI32>,
        io_map: Arc<Mutex<IOMap>>,
        buffer_queue_sender: Sender<Vec<TxMessage>>,
        send_queue_receiver: Receiver<Vec<TxMessage>>,
        sleeper: S,
        thread_priority: ThreadPriority,
        #[cfg(target_os = "windows")] process_priority: super::ProcessPriority,
        ec_send_cycle: Duration,
    ) -> Self {
        Self {
            ecatth_handle: Some(std::thread::spawn(move || {
                Self::ecat_run::<S>(
                    is_open,
                    io_map,
                    wkc,
                    buffer_queue_sender,
                    send_queue_receiver,
                    ec_send_cycle,
                    sleeper,
                    thread_priority,
                    #[cfg(target_os = "windows")]
                    process_priority,
                )
            })),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ecat_run<S: Sleep>(
        is_open: Arc<AtomicBool>,
        io_map: Arc<Mutex<IOMap>>,
        wkc: Arc<AtomicI32>,
        buffer_queue_sender: Sender<Vec<TxMessage>>,
        receiver: Receiver<Vec<TxMessage>>,
        cycle: Duration,
        sleeper: S,
        thread_priority: ThreadPriority,
        #[cfg(target_os = "windows")] process_priority: super::ProcessPriority,
    ) -> Result<(), SOEMError> {
        unsafe {
            #[cfg(target_os = "windows")]
            let old_priority = {
                let old_priority = windows::Win32::System::Threading::GetPriorityClass(
                    windows::Win32::System::Threading::GetCurrentProcess(),
                );
                if let Err(e) = windows::Win32::System::Threading::SetPriorityClass(
                    windows::Win32::System::Threading::GetCurrentProcess(),
                    process_priority.into(),
                ) {
                    tracing::warn!(
                        "Failed to set process priority to {:?}: {:?}.",
                        process_priority,
                        e
                    );
                }
                old_priority
            };
            thread_priority.set_for_current()?;

            let mut cnt_miss_deadline = 0;
            let mut toff = time::Duration::ZERO;
            let mut integral = 0;
            ec_send_processdata();
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
                    ec_receive_processdata(EC_TIMEOUTRET as i32),
                    Ordering::Relaxed,
                );

                toff = Self::ec_sync(ec_DCtime, cycle.as_nanos() as _, &mut integral);

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
                ec_send_processdata();
            }

            #[cfg(target_os = "windows")]
            {
                if let Err(e) = windows::Win32::System::Threading::SetPriorityClass(
                    windows::Win32::System::Threading::GetCurrentProcess(),
                    windows::Win32::System::Threading::PROCESS_CREATION_FLAGS(old_priority),
                ) {
                    tracing::warn!(
                        "Failed to restore process priority to {:?}: {:?}.",
                        old_priority,
                        e
                    );
                }
            }
        }
        Ok(())
    }

    fn ec_sync(reftime: i64, cycletime: i64, integral: &mut i64) -> time::Duration {
        let mut delta = (reftime - 50000) % cycletime;
        if delta > (cycletime / 2) {
            delta -= cycletime;
        }
        if delta > 0 {
            *integral += 1;
        }
        if delta < 0 {
            *integral -= 1;
        }
        (-(delta / 100) - (*integral / 20)).nanoseconds()
    }
}

impl Drop for SOEMECatThreadGuard {
    fn drop(&mut self) {
        if let Some(timer) = self.ecatth_handle.take() {
            let _ = timer.join();
        }
    }
}

struct SOEMEcatCheckThreadGuard {
    ecat_check_th: Option<JoinHandle<()>>,
}

impl SOEMEcatCheckThreadGuard {
    fn new<F: Fn(usize, Status) + Send + Sync + 'static>(
        is_open: Arc<AtomicBool>,
        err_handler: F,
        wkc: Arc<AtomicI32>,
        state_check_interval: Duration,
    ) -> Self {
        let expected_wkc = unsafe { (ec_group[0].outputsWKC * 2 + ec_group[0].inputsWKC) as i32 };
        Self {
            ecat_check_th: Some(std::thread::spawn(move || {
                let err_handler = EcatErrorHandler { err_handler };
                err_handler.run(is_open, wkc, expected_wkc, state_check_interval)
            })),
        }
    }
}

impl Drop for SOEMEcatCheckThreadGuard {
    fn drop(&mut self) {
        if let Some(th) = self.ecat_check_th.take() {
            let _ = th.join();
        }
    }
}

#[cfg(feature = "async")]
use autd3_core::link::AsyncLink;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
#[cfg_attr(feature = "async-trait", autd3_core::async_trait)]
impl<F: Fn(usize, Status) + Send + Sync + 'static, S: Sleep + Send + 'static> AsyncLink
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
