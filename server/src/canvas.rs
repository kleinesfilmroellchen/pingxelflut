use concurrent_queue::ConcurrentQueue;
use parking_lot::RwLock;
use std::sync::Arc;

use pixels::Pixels;
use rgb::RGBA8;

type Color = RGBA8;
const COLOR_SIZE: usize = 4;

pub fn to_internal_color(color: pingxelflut::format::Color) -> Color {
    Color::new(color.red, color.green, color.blue, color.alpha())
}

/// Canvas handling datastructures.
/// This is a lightweight, easily clonable datastructure that contains reference-counted references to the underlying shared data, such as the frame buffer and pixel queue.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub(crate) pixels: Arc<RwLock<Pixels>>,
    pub(crate) pixel_queue: Arc<ConcurrentQueue<(usize, Color)>>,
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
        let _ = self.pixel_queue.force_push((pixel_pos, color));
    }

    /// Sets all the pixels from the queue.
    pub fn set_queue_pixels(&self) {
        let mut pixels = self.pixels.write();
        let frame = pixels.frame_mut();
        while let Ok((pixel_pos, color)) = self.pixel_queue.pop() {
            let pixel_end_pos = pixel_pos + COLOR_SIZE;
            frame[pixel_pos..pixel_end_pos].copy_from_slice(color.as_ref());
        }
    }
}
