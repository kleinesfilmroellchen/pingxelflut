use parking_lot::RwLock;
use std::sync::Arc;

use pixels::Pixels;
use rgb::RGBA8;

type Color = RGBA8;
const COLOR_SIZE: usize = 4;

pub fn to_internal_color(color: pingxelflut::format::Color) -> Color {
    Color::new(color.red, color.green, color.blue, color.alpha())
}

#[derive(Debug, Clone)]
pub struct Canvas {
    pub(crate) pixels: Arc<RwLock<Pixels>>,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl Canvas {
    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color) {
        let x = x as usize;
        let y = y as usize;
        if x > self.width as usize || y > self.height as usize {
            return;
        }
        let pixel_pos = (x + y * self.width as usize) * COLOR_SIZE;
        let pixel_end_pos = pixel_pos + COLOR_SIZE as usize;
        {
            let mut pixels = self.pixels.write();
            pixels.frame_mut()[pixel_pos..pixel_end_pos].copy_from_slice(color.as_ref());
        }
    }
}
