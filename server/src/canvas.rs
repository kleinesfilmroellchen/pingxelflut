use async_channel::{Receiver, Sender};
use parking_lot::RwLock;
use pingxelflut::format::{Color, COLOR_SIZE};
use rgb::ComponentSlice;
use std::sync::Arc;

use pixels::Pixels;

/// Canvas handling datastructures.
/// This is a lightweight, easily clonable datastructure that contains reference-counted references to the underlying shared data, such as the frame buffer and pixel queue.
#[derive(Debug, Clone)]
pub struct Canvas {
    pub(crate) pixels: Arc<RwLock<Pixels>>,
    pub(crate) pixel_queue_in: Sender<(usize, Color)>,
    pub(crate) pixel_queue_out: Receiver<(usize, Color)>,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl Canvas {
    pub fn new(pixels: Arc<RwLock<Pixels>>, width: u16, height: u16) -> Self {
        let (pixel_queue_in, pixel_queue_out) = async_channel::unbounded();
        Self {
            pixels,
            pixel_queue_in,
            pixel_queue_out,
            width,
            height,
        }
    }

    pub fn set_pixel(&mut self, x: u16, y: u16, color: Color) {
        if color.a == 0 {
            return;
        }
        let x = x as usize;
        let y = y as usize;
        if x >= self.width as usize || y >= self.height as usize {
            return;
        }
        let pixel_pos = (x + y * self.width as usize) * COLOR_SIZE;

        let _ = self.pixel_queue_in.force_send((pixel_pos, color));
    }

    /// Sets all the pixels from the queue.
    pub fn set_queue_pixels(&self) {
        let mut pixels = self.pixels.write();
        let frame = pixels.frame_mut();
        while let Ok((pixel_pos, color)) = self.pixel_queue_out.try_recv() {
            let pixel_end_pos = pixel_pos + COLOR_SIZE;
            frame[pixel_pos..pixel_end_pos].copy_from_slice(color.as_slice());
        }
    }
}
