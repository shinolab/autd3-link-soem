use anyhow::Result;

use autd3::prelude::*;
use autd3_link_soem::RemoteSOEM;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut autd = Controller::builder([AUTD3::new(Point3::origin())])
        .open(RemoteSOEM::builder("127.0.0.1:8080".parse()?))?;

    autd.send((
        Sine::new(150. * Hz),
        Focus::new(autd.center() + Vector3::new(0., 0., 150. * mm)),
    ))?;

    println!("Press Enter to quit.");
    let mut _s = String::new();
    std::io::stdin().read_line(&mut _s)?;

    autd.close()?;

    Ok(())
}
