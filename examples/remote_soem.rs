use anyhow::Result;

use autd3::prelude::*;
use autd3_link_soem::RemoteSOEM;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut autd = Controller::builder([AUTD3::new(Vector3::zeros())])
        .open(RemoteSOEM::builder("127.0.0.1:8080".parse()?))
        .await?;

    autd.send((
        Sine::new(150. * Hz),
        Focus::new(autd.center() + Vector3::new(0., 0., 150. * mm)),
    ))
    .await?;

    autd.close().await?;

    Ok(())
}
