use anyhow::Result;

use autd3::prelude::*;
use autd3_link_soem::{SOEM, Status};

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let mut autd = Controller::open(
        [AUTD3 {
            pos: Point3::origin(),
            rot: UnitQuaternion::identity(),
        }],
        SOEM::new(
            |slave, status| {
                eprintln!("slave[{}]: {}", slave, status);
                if status == Status::Lost {
                    // You can also wait for the link to recover, without exitting the process
                    std::process::exit(-1);
                }
            },
            Default::default(),
        ),
    )?;

    autd.send((
        Sine {
            freq: 150. * Hz,
            option: Default::default(),
        },
        Focus {
            pos: autd.center() + Vector3::new(0., 0., 150. * mm),
            option: Default::default(),
        },
    ))?;

    println!("Press Enter to quit.");
    let mut _s = String::new();
    std::io::stdin().read_line(&mut _s)?;

    autd.close()?;

    Ok(())
}
