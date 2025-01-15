use anyhow::Result;

use autd3::prelude::*;
use autd3_link_soem::{Status, SOEM};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut autd = Controller::builder([AUTD3::new(Point3::origin())]).open(
        SOEM::builder().with_err_handler(|slave, status| {
            eprintln!("slave[{}]: {}", slave, status);
            if status == Status::Lost {
                // You can also wait for the link to recover, without exitting the process
                std::process::exit(-1);
            }
        }),
    )?;

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
