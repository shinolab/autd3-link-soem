use std::{num::NonZeroUsize, time::Duration};

use super::{error_handler::Status, sync_mode::SyncMode, timer_strategy::TimerStrategy, SOEM};

use autd3_core::{
    ethercat::EC_CYCLE_TIME_BASE,
    geometry::Geometry,
    link::{LinkBuilder, LinkError},
};

use derive_more::Debug;

use thread_priority::ThreadPriority;

/// A option for [`SOEM`].
#[derive(Debug)]
pub struct SOEMOption {
    /// The size of the send queue buffer. The default is 32.
    pub buf_size: NonZeroUsize,
    /// The timer strategy. The default is [`TimerStrategy::SpinSleep`].
    pub timer_strategy: TimerStrategy,
    /// The synchronization mode. The default is [`SyncMode::DC`].
    pub sync_mode: SyncMode,
    /// The network interface name. If this is empty, the network interface will be automatically selected to which the AUTD3 device is connected. The default is empty.
    pub ifname: String,
    /// The interval to check the state. The default is 100ms.
    pub state_check_interval: Duration,
    /// The cycle of the sync0 signal. The value must be a multiple of [`EC_CYCLE_TIME_BASE`] and not be zero. The default is 1ms.
    pub sync0_cycle: Duration,
    /// The send cycle. The value must be a multiple of [`EC_CYCLE_TIME_BASE`] and not be zero. The default is 1ms.
    pub send_cycle: Duration,
    /// The thread priority. The default is [`ThreadPriority::Max`].
    pub thread_priority: ThreadPriority,
    #[cfg(target_os = "windows")]
    /// The process priority. The default is [`super::ProcessPriority::High`].
    pub process_priority: super::ProcessPriority,
    /// The synchronization tolerance. The default is 1us.
    pub sync_tolerance: Duration,
    /// The synchronization timeout. The default is 10s.
    pub sync_timeout: Duration,
}

impl Default for SOEMOption {
    fn default() -> Self {
        Self {
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
            sync_tolerance: std::time::Duration::from_micros(1),
            sync_timeout: std::time::Duration::from_secs(10),
        }
    }
}

/// A builder for [`SOEM`].
#[derive(Debug)]
pub struct SOEMBuilder<F: Fn(usize, Status) + Send + Sync + 'static> {
    pub(crate) option: SOEMOption,
    #[debug(skip)]
    /// The error handler which is called when an error occurs. The default is `None`.
    pub(crate) err_handler: F,
}

impl<F: Fn(usize, Status) + Send + Sync + 'static> LinkBuilder for SOEMBuilder<F> {
    type L = SOEM;

    fn open(self, geometry: &Geometry) -> Result<Self::L, LinkError> {
        Self::L::open(self, geometry)
    }
}

#[cfg(feature = "async")]
use autd3_core::link::AsyncLinkBuilder;

#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
#[cfg_attr(feature = "async-trait", autd3_core::async_trait)]
impl<F: Fn(usize, Status) + Send + Sync + 'static> AsyncLinkBuilder for SOEMBuilder<F> {
    type L = SOEM;

    async fn open(self, geometry: &Geometry) -> Result<Self::L, LinkError> {
        <Self as LinkBuilder>::open(self, geometry)
    }
}
