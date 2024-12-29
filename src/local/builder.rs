use std::{num::NonZeroUsize, time::Duration};

use super::{
    error_handler::{ErrHandler, Status},
    timer_strategy::TimerStrategy,
    SyncMode, SOEM,
};

use autd3_driver::{derive::*, ethercat::EC_CYCLE_TIME_BASE, link::LinkBuilder};

use derive_more::Debug;

use thread_priority::ThreadPriority;

/// A builder for [`SOEM`].
#[derive(Builder, Debug)]
pub struct SOEMBuilder {
    #[get]
    #[set]
    /// The size of the send queue buffer. The default is 32.
    pub(crate) buf_size: NonZeroUsize,
    #[get]
    #[set]
    /// The timer strategy. The default is [`TimerStrategy::SpinSleep`].
    pub(crate) timer_strategy: TimerStrategy,
    #[get]
    #[set]
    /// The synchronization mode. The default is [`SyncMode::DC`].
    pub(crate) sync_mode: SyncMode,
    #[get(ref)]
    #[set(into)]
    /// The network interface name. If this is empty, the network interface will be automatically selected to which the AUTD3 device is connected. The default is empty.
    pub(crate) ifname: String,
    #[get]
    #[set]
    /// The interval to check the state. The default is 100ms.
    pub(crate) state_check_interval: Duration,
    #[get]
    #[set]
    /// The cycle of the sync0 signal. The value must be a multiple of [`EC_CYCLE_TIME_BASE`] and not be zero. The default is 1ms.
    pub(crate) sync0_cycle: Duration,
    #[get]
    #[set]
    /// The send cycle. The value must be a multiple of [`EC_CYCLE_TIME_BASE`] and not be zero. The default is 1ms.
    pub(crate) send_cycle: Duration,
    #[get]
    #[set]
    /// The thread priority. The default is [`ThreadPriority::Max`].
    pub(crate) thread_priority: ThreadPriority,
    #[cfg(target_os = "windows")]
    #[get]
    #[set]
    /// The process priority. The default is [`super::ProcessPriority::High`].
    pub(crate) process_priority: super::ProcessPriority,
    #[debug(skip)]
    /// The error handler which is called when an error occurs. The default is `None`.
    pub(crate) err_handler: Option<ErrHandler>,
    #[get]
    #[set]
    /// The synchronization tolerance. The default is 1us.
    pub(crate) sync_tolerance: Duration,
    #[get]
    #[set]
    /// The synchronization timeout. The default is 10s.
    pub(crate) sync_timeout: Duration,
}

impl Default for SOEMBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl SOEMBuilder {
    pub(crate) fn new() -> Self {
        SOEMBuilder {
            buf_size: NonZeroUsize::new(32).unwrap(),
            timer_strategy: TimerStrategy::SpinSleep,
            sync_mode: SyncMode::DC,
            ifname: String::new(),
            state_check_interval: Duration::from_millis(100),
            sync0_cycle: EC_CYCLE_TIME_BASE * 2,
            send_cycle: EC_CYCLE_TIME_BASE * 2,
            thread_priority: ThreadPriority::Max,
            #[cfg(target_os = "windows")]
            process_priority: super::ProcessPriority::High,
            err_handler: None,
            sync_tolerance: std::time::Duration::from_micros(1),
            sync_timeout: std::time::Duration::from_secs(10),
        }
    }

    /// Set the `err_handler` field.
    pub fn with_err_handler(
        self,
        err_handler: impl Fn(usize, Status) + Send + Sync + 'static,
    ) -> Self {
        Self {
            err_handler: Some(Box::new(err_handler)),
            ..self
        }
    }
}

#[cfg_attr(feature = "async-trait", autd3_driver::async_trait)]
impl LinkBuilder for SOEMBuilder {
    type L = SOEM;

    async fn open(
        self,
        geometry: &autd3_driver::geometry::Geometry,
    ) -> Result<Self::L, AUTDDriverError> {
        Self::L::open(self, geometry).await
    }
}
