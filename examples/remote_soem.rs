use anyhow::Result;

use autd3::prelude::*;
use autd3_link_soem::RemoteSOEM;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut autd = autd3::r#async::Controller::open(
        [AUTD3 {
            pos: Point3::origin(),
            rot: UnitQuaternion::identity(),
        }],
        RemoteSOEM::new("127.0.0.1:8080".parse()?),
    )
    .await?;

    autd.send((
        Sine {
            freq: 150. * Hz,
            option: Default::default(),
        },
        Focus {
            pos: autd.center() + Vector3::new(0., 0., 150. * mm),
            option: Default::default(),
        },
    ))
    .await?;

    println!("Press Enter to quit.");
    let mut _s = String::new();
    std::io::stdin().read_line(&mut _s)?;

    autd.close().await?;

    Ok(())
}
