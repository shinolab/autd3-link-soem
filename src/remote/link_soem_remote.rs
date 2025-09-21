use std::net::SocketAddr;

use autd3_core::{
    geometry::Geometry,
    link::{AsyncLink, LinkError, RxMessage, TxBufferPoolSync, TxMessage},
};
use autd3_protobuf::*;

pub struct RemoteSOEMInner {
    client: ecat_client::EcatClient<tonic::transport::Channel>,
    buffer_pool: TxBufferPoolSync,
}

impl RemoteSOEMInner {
    async fn open(addr: &SocketAddr, geometry: &Geometry) -> Result<Self, LinkError> {
        tracing::info!("Connecting to remote SOEM server@{}", addr);

        let conn = tonic::transport::Endpoint::new(format!("http://{addr}"))
            .map_err(AUTDProtoBufError::from)?
            .connect()
            .await
            .map_err(AUTDProtoBufError::from)?;

        let mut buffer_pool = TxBufferPoolSync::new();
        buffer_pool.init(geometry);

        Ok(Self {
            client: ecat_client::EcatClient::new(conn),
            buffer_pool,
        })
    }

    async fn close(&mut self) -> Result<(), LinkError> {
        self.client
            .close(CloseRequest {})
            .await
            .map_err(AUTDProtoBufError::from)?;
        Ok(())
    }

    async fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, LinkError> {
        Ok(self.buffer_pool.borrow())
    }

    async fn send(&mut self, tx: Vec<TxMessage>) -> Result<(), LinkError> {
        let tx_data = TxRawData::from(tx.as_slice());
        self.buffer_pool.return_buffer(tx);
        self.client
            .send_data(tx_data)
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

impl AsyncLink for RemoteSOEM {
    async fn open(&mut self, geometry: &Geometry) -> Result<(), LinkError> {
        self.inner = Some(RemoteSOEMInner::open(&self.addr, geometry).await?);
        Ok(())
    }

    async fn close(&mut self) -> Result<(), LinkError> {
        if let Some(mut inner) = self.inner.take() {
            inner.close().await?;
        }
        Ok(())
    }

    async fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, LinkError> {
        if let Some(inner) = self.inner.as_mut() {
            inner.alloc_tx_buffer().await
        } else {
            Err(LinkError::closed())
        }
    }

    async fn send(&mut self, tx: Vec<TxMessage>) -> Result<(), LinkError> {
        if let Some(inner) = self.inner.as_mut() {
            inner.send(tx).await
        } else {
            Err(LinkError::closed())
        }
    }

    async fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        if let Some(inner) = self.inner.as_mut() {
            inner.receive(rx).await
        } else {
            Err(LinkError::closed())
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

    fn send(&mut self, tx: Vec<TxMessage>) -> Result<(), LinkError> {
        self.runtime
            .as_ref()
            .map_or(Err(LinkError::closed()), |runtime| {
                runtime.block_on(async {
                    if let Some(inner) = self.inner.as_mut() {
                        inner.send(tx).await
                    } else {
                        Err(LinkError::closed())
                    }
                })
            })
    }

    fn receive(&mut self, rx: &mut [RxMessage]) -> Result<(), LinkError> {
        self.runtime
            .as_ref()
            .map_or(Err(LinkError::closed()), |runtime| {
                runtime.block_on(async {
                    if let Some(inner) = self.inner.as_mut() {
                        inner.receive(rx).await
                    } else {
                        Err(LinkError::closed())
                    }
                })
            })
    }

    fn is_open(&self) -> bool {
        self.runtime.is_some() && self.inner.is_some()
    }

    fn alloc_tx_buffer(&mut self) -> Result<Vec<TxMessage>, LinkError> {
        self.runtime
            .as_ref()
            .map_or(Err(LinkError::closed()), |runtime| {
                runtime.block_on(async {
                    if let Some(inner) = self.inner.as_mut() {
                        inner.alloc_tx_buffer().await
                    } else {
                        Err(LinkError::closed())
                    }
                })
            })
    }
}
