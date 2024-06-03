use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pingxelflut::get_size;

/// A simple Pingxelflut client.
#[derive(Clone, Parser, Debug)]
struct Arguments {
    /// Target server to send pixels to.
    #[arg(short, long, value_name = "ADDRESS")]
    target: IpAddr,
    /// Source image to send.
    #[arg(short, long, value_name = "IMAGE")]
    image: PathBuf,
    /// X offset to send image at.
    #[arg(short, value_name = "X", default_value = "0")]
    x: u16,
    /// Y offset to send image at.
    #[arg(short, value_name = "Y", default_value = "0")]
    y: u16,
}

fn main() -> Result<()> {
    let arguments: Arguments = Parser::parse();
    let image = image::open(arguments.image)?;
    let size = get_size(arguments.target)?;

    Ok(())
}
