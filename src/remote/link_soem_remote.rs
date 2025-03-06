use std::net::SocketAddr;

use autd3_core::{
    geometry::Geometry,
    link::{AsyncLink, LinkError, RxMessage, TxMessage},
};
use autd3_protobuf::*;

pub struct RemoteSOEMInner {
    client: ecat_client::EcatClient<tonic::transport::Channel>,
}

impl RemoteSOEMInner {
    async fn open(addr: &SocketAddr) -> Result<Self, LinkError> {
        tracing::info!("Connecting to remote SOEM server@{}", addr);

        let conn = tonic::transport::Endpoint::new(format!("http://{}", addr))
            .map_err(AUTDProtoBufError::from)?
            .connect()
            .await
            .map_err(AUTDProtoBufError::from)?;

        Ok(Self {
            client: ecat_client::EcatClient::new(conn),
        })
    }

    async fn close(&mut self) -> Result<(), LinkError> {
        self.client
            .close(CloseRequest {})
            .await
            .map_err(AUTDProtoBufError::from)?;
        Ok(())
    }

    async fn send(&mut self, tx: &[TxMessage]) -> Result<(), LinkError> {
        self.client
            .send_data(TxRawData::from(tx))
            .await
            .map_err(AUTDProtoBufError::from)?;
        Ok(())
    }

    async fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        let rx_ = Vec::<RxMessage>::from_msg(
            self.client
                .read_data(ReadRequest {})
                .await
                .map_err(AUTDProtoBufError::from)?
                .into_inner(),
        )?;
        rx.copy_from_slice(&rx_);
        Ok(())
    }
}

/// An [`AsyncLink`] using [SOEM] on a remote server.
///
/// To use this link, you need to run [`SOEMAUTDServer`] on the remote server before.
///
/// [SOEM]: https://github.com/OpenEtherCATsociety/SOEM
/// [`SOEMAUTDServer`]: https://github.com/shinolab/autd3-server
pub struct RemoteSOEM {
    addr: SocketAddr,
    inner: Option<RemoteSOEMInner>,
    #[cfg(feature = "blocking")]
    runtime: Option<tokio::runtime::Runtime>,
}

impl RemoteSOEM {
    /// Create a new [`RemoteSOEM`].
    pub const fn new(addr: SocketAddr) -> RemoteSOEM {
        RemoteSOEM {
            addr,
            inner: None,
            #[cfg(feature = "blocking")]
            runtime: None,
        }
    }
}

#[cfg_attr(feature = "async-trait", autd3_core::async_trait)]
impl AsyncLink for RemoteSOEM {
    async fn open(&mut self, _: &Geometry) -> Result<(), LinkError> {
        self.inner = Some(RemoteSOEMInner::open(&self.addr).await?);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), LinkError> {
        if let Some(mut inner) = self.inner.take() {
            inner.close().await?;
        }
        Ok(())
    }

    async fn send(&mut self, tx: &[TxMessage]) -> Result<(), LinkError> {
        if let Some(inner) = self.inner.as_mut() {
            inner.send(tx).await
        } else {
            Err(LinkError::new("Link is closed.".to_owned()))
        }
    }

    async fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        if let Some(inner) = self.inner.as_mut() {
            inner.receive(rx).await
        } else {
            Err(LinkError::new("Link is closed.".to_owned()))
        }
    }

    fn is_open(&self) -> bool {
        self.inner.is_some()
    }
}

#[cfg(feature = "blocking")]
use autd3_core::link::Link;

#[cfg(feature = "blocking")]
impl Link for RemoteSOEM {
    fn open(&mut self, geometry: &Geometry) -> Result<(), LinkError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");
        runtime.block_on(<Self as AsyncLink>::open(self, geometry))?;
        self.runtime = Some(runtime);
        Ok(())
    }

    fn close(&mut self) -> Result<(), LinkError> {
        self.runtime.as_ref().map_or(Ok(()), |runtime| {
            runtime.block_on(async {
                if let Some(mut inner) = self.inner.take() {
                    inner.close().await?;
                }
                Ok(())
            })
        })
    }

    fn send(&mut self, tx: &[TxMessage]) -> Result<(), LinkError> {
        self.runtime.as_ref().map_or(
            Err(LinkError::new("Link is closed.".to_owned())),
            |runtime| {
                runtime.block_on(async {
                    if let Some(inner) = self.inner.as_mut() {
                        inner.send(tx).await?;
                        Ok(())
                    } else {
                        Err(LinkError::new("Link is closed.".to_owned()))
                    }
                })
            },
        )
    }

    fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        self.runtime.as_ref().map_or(
            Err(LinkError::new("Link is closed.".to_owned())),
            |runtime| {
                runtime.block_on(async {
                    if let Some(inner) = self.inner.as_mut() {
                        inner.receive(rx).await?;
                        Ok(())
                    } else {
                        Err(LinkError::new("Link is closed.".to_owned()))
                    }
                })
            },
        )
    }

    fn is_open(&self) -> bool {
        self.runtime.is_some() && self.inner.is_some()
    }
}
