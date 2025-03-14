use clap::Parser;
use r2dma::{ib::GidType, DeviceConfig, Result};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// device filter by name.
    #[arg(long)]
    pub device_filter: Vec<String>,

    /// enable gid type filter (IB or RoCE v2).
    #[arg(long, default_value_t = false)]
    pub gid_type_filter: bool,

    /// RoCE v2 skip link local address.
    #[arg(long, default_value_t = false)]
    pub skip_link_local_addr: bool,

    /// enable verbose logging.
    #[arg(long, short, default_value_t = false)]
    pub verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_max_level(if args.verbose {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .init();

    let mut config = DeviceConfig::default();
    config.device_filter.extend(args.device_filter);
    if args.gid_type_filter {
        config.gid_type_filter = [GidType::IB, GidType::RoCEv2].into();
    }
    if args.skip_link_local_addr {
        config.roce_v2_skip_link_local_addr = true;
    }

    let devices = r2dma::Device::avaiables(&config)?;
    for device in devices {
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
