use clap::Parser;
use r2dma::*;

#[derive(Parser, Debug)]
struct Args {}

fn main() -> Result<()> {
    let _ = Args::parse();

    let device_list = DeviceList::available()?;
    for device in device_list.iter() {
        println!("device: {:#?}", device);

        let context = Context::create(device)?;
        let device_attr = context.query_device()?;
        for port_num in 1..=device_attr.phys_port_cnt {
            let port_attr = context.query_port(port_num)?;
            println!("port {port_num}: {:#?}", port_attr);

            for gid_index in 0..port_attr.gid_tbl_len {
                if let Ok(entry) = context.query_gid(port_num, gid_index as u16) {
                    println!("{gid_index}: {:?}", entry);
                }
            }
        }
    }

    Ok(())
}
