use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use image::DynamicImage;
use image::GenericImageView;
use pingxelflut::format::color_from_rgba;
use pingxelflut::get_size;
use pingxelflut::set_pixel;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;

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
    /// Whether to request the canvas size prior to sending.
    /// This may make the client work better via localhost and on Windows.
    /// By default, 1920x1080 is used.
    #[arg(long)]
    no_request_size: bool,
}

fn send_pixel_from_image(
    image: &DynamicImage,
    target: IpAddr,
    x: u16,
    y: u16,
    offset_x: u16,
    offset_y: u16,
) -> Result<()> {
    let pixel = image.get_pixel(x.into(), y.into());
    set_pixel(target, x + offset_x, y + offset_y, color_from_rgba(pixel.0))?;
    Ok(())
}

fn main() -> Result<()> {
    let arguments: Arguments = Parser::parse();
    let mut image = image::open(arguments.image)?;
    let (width, height) = if arguments.no_request_size {
        (1920u16, 1080u16)
    } else {
        get_size(arguments.target)?
    };

    image = image.crop_imm(
        0,
        0,
        image.width().min(width.into()),
        image.height().min(height.into()),
    );

    loop {
        (0..(image.width() as u16)).into_par_iter().for_each(|x| {
            for y in 0..(image.height() as u16) {
                let result =
                    send_pixel_from_image(&image, arguments.target, x, y, arguments.x, arguments.y);
                if let Err(err) = result {
                    eprintln!("error while sending pixel: {:?}", err);
                }
            }
        });
    }
}
