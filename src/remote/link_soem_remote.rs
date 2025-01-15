use std::net::SocketAddr;

use autd3_core::{
    geometry::Geometry,
    link::{AsyncLink, AsyncLinkBuilder, LinkError, RxMessage, TxMessage},
};
use autd3_protobuf::*;

/// An [`AsyncLink`] using [SOEM] on a remote server.
///
/// To use this link, you need to run [`SOEMAUTDServer`] on the remote server before.
///
/// [SOEM]: https://github.com/OpenEtherCATsociety/SOEM
/// [`SOEMAUTDServer`]: https://github.com/shinolab/autd3-server
pub struct RemoteSOEM {
    client: ecat_client::EcatClient<tonic::transport::Channel>,
    is_open: bool,
}

/// A builder for [`RemoteSOEM`].
#[derive(Debug)]
pub struct RemoteSOEMBuilder {
    addr: SocketAddr,
}

#[cfg_attr(feature = "async-trait", autd3_core::async_trait)]
impl AsyncLinkBuilder for RemoteSOEMBuilder {
    type L = RemoteSOEM;

    #[tracing::instrument(level = "debug", skip(_geometry))]
    async fn open(self, _geometry: &Geometry) -> Result<Self::L, LinkError> {
        tracing::info!("Connecting to remote SOEM server@{}", self.addr);

        let conn = tonic::transport::Endpoint::new(format!("http://{}", self.addr))
            .map_err(AUTDProtoBufError::from)?
            .connect()
            .await
            .map_err(AUTDProtoBufError::from)?;
        Ok(Self::L {
            client: ecat_client::EcatClient::new(conn),
            is_open: true,
        })
    }
}

impl RemoteSOEM {
    /// Create a new [`RemoteSOEM`] builder.
    pub const fn builder(addr: SocketAddr) -> RemoteSOEMBuilder {
        RemoteSOEMBuilder { addr }
    }
}

#[cfg_attr(feature = "async-trait", autd3_core::async_trait)]
impl AsyncLink for RemoteSOEM {
    async fn close(&mut self) -> Result<(), LinkError> {
        self.is_open = false;
        self.client
            .close(CloseRequest {})
            .await
            .map_err(AUTDProtoBufError::from)?;
        Ok(())
    }

    async fn send(&mut self, tx: &[TxMessage]) -> Result<bool, LinkError> {
        Ok(self
            .client
            .send_data(tx.to_msg(None))
            .await
            .map_err(AUTDProtoBufError::from)?
            .into_inner()
            .success)
    }

    async fn receive(&mut self, rx: &mut [RxMessage]) -> Result<bool, LinkError> {
        let rx_ = Vec::<RxMessage>::from_msg(
            &self
                .client
                .read_data(ReadRequest {})
                .await
                .map_err(AUTDProtoBufError::from)?
                .into_inner(),
        )?;
        rx.copy_from_slice(&rx_);

        Ok(true)
    }

    fn is_open(&self) -> bool {
        self.is_open
    }
}

#[cfg(feature = "blocking")]
use autd3_core::link::{Link, LinkBuilder};

/// A [`Link`] using [SOEM] on a remote server.
///
/// To use this link, you need to run [`SOEMAUTDServer`] on the remote server before.
///
/// [SOEM]: https://github.com/OpenEtherCATsociety/SOEM
/// [`SOEMAUTDServer`]: https://github.com/shinolab/autd3-server
#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
#[cfg(feature = "blocking")]
pub struct RemoteSOEMBlocking {
    runtime: tokio::runtime::Runtime,
    inner: RemoteSOEM,
}

#[cfg(feature = "blocking")]
impl Link for RemoteSOEMBlocking {
    fn close(&mut self) -> Result<(), LinkError> {
        self.runtime.block_on(self.inner.close())
    }

    fn update(&mut self, geometry: &autd3_core::geometry::Geometry) -> Result<(), LinkError> {
        self.runtime.block_on(self.inner.update(geometry))
    }

    fn send(&mut self, tx: &[TxMessage]) -> Result<bool, LinkError> {
        self.runtime.block_on(self.inner.send(tx))
    }

    fn receive(&mut self, rx: &mut [RxMessage]) -> Result<bool, LinkError> {
        self.runtime.block_on(self.inner.receive(rx))
    }

    fn is_open(&self) -> bool {
        self.inner.is_open()
    }

    fn trace(&mut self, timeout: Option<std::time::Duration>, parallel_threshold: Option<usize>) {
        self.inner.trace(timeout, parallel_threshold)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "blocking")))]
#[cfg(feature = "blocking")]
impl LinkBuilder for RemoteSOEMBuilder {
    type L = RemoteSOEMBlocking;

    fn open(self, geometry: &autd3_core::geometry::Geometry) -> Result<Self::L, LinkError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        let inner = runtime.block_on(<Self as AsyncLinkBuilder>::open(self, geometry))?;
        Ok(Self::L { runtime, inner })
    }
}
