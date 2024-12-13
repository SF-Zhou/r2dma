use clap::Parser;
use r2dma::{ibv::GidType, Result};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {}

fn main() -> Result<()> {
    let _ = Args::parse();

    let devices = r2dma::Devices::open()?;
    for device in devices.iter() {
        println!("device: {:#?}", device.context().device());

        for port in device.ports() {
            println!("port {}: {:#?}", port.port_num, port.port_attr);
            for (gid_index, gid, gid_type) in &port.gids {
                match gid_type {
                    GidType::IB => {
                        println!("{gid_index}: {:?}, InfiniBand", gid);
                    }
                    GidType::RoCEv1 => {
                        println!("{gid_index}: {:?}, RoCE v1", gid);
                    }
                    GidType::RoCEv2 => {
                        println!("{gid_index}: {}, RoCE v2", gid.as_ipv6())
                    }
                    GidType::Other(t) => {
                        println!("{gid_index}: {:?}, {}", gid, t)
                    }
                }
            }
        }
    }

    Ok(())
}
