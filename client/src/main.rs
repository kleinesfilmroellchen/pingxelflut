use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use image::DynamicImage;
use image::GenericImageView;
use image::Pixel;
use pingxelflut::format::Color;
use pingxelflut::get_size;
use pingxelflut::set_pixel;

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

/// Check whether an image has transparency.
fn image_has_transparency(image: &DynamicImage) -> bool {
    image.as_rgb8().is_none()
        || image.as_luma8().is_none()
        || image.as_rgb16().is_none()
        || image.as_rgb32f().is_none()
        || image.as_luma16().is_none()
}

fn send_pixel_from_image(
    image: &DynamicImage,
    target: IpAddr,
    has_transparency: bool,
    x: u16,
    y: u16,
) -> Result<()> {
    let pixel = image.get_pixel(x.into(), y.into());
    let color = if has_transparency {
        Color::from_rgba(pixel.to_rgba().0)
    } else {
        Color::from_rgb(pixel.to_rgb().0)
    };

    set_pixel(target, x, y, color)?;
    Ok(())
}

fn main() -> Result<()> {
    let arguments: Arguments = Parser::parse();
    let mut image = image::open(arguments.image)?;
    let (width, height) = get_size(arguments.target)?;

    image = image.crop_imm(
        0,
        0,
        image.width().min(width.into()),
        image.height().min(height.into()),
    );
    let has_transparency = image_has_transparency(&image);

    loop {
        for x in 0..(image.width() as u16) {
            for y in 0..(image.height() as u16) {
                let result =
                    send_pixel_from_image(&image, arguments.target, has_transparency, x, y);
                if let Err(err) = result {
                    eprintln!("error while sending pixel: {:?}", err);
                }
            }
        }
    }
}
