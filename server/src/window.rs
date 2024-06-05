use std::sync::Arc;

use crate::{canvas::Canvas, ping_handler};
use log::error;
use parking_lot::RwLock;
use pixels::{wgpu::Color, Pixels, SurfaceTexture};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

pub struct App {
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    pixels: Option<Arc<RwLock<Pixels>>>,
    canvas: Option<Canvas>,
    width: u16,
    height: u16,
}

impl App {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            window_id: None,
            window: None,
            pixels: None,
            canvas: None,
            width,
            height,
        }
    }
}

impl ApplicationHandler for App {
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Pingxelflut")
            .with_inner_size(winit::dpi::PhysicalSize::new(self.width, self.height));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window_id = Some(window.id());
        self.window = Some(window.clone());

        let window = self.window.as_ref().unwrap().clone();
        let mut pixels = {
            let surface_texture =
                SurfaceTexture::new(self.width as u32, self.height as u32, &window);
            Pixels::new(self.width as u32, self.height as u32, surface_texture).unwrap()
        };
        pixels.clear_color(Color::BLACK);
        self.pixels = Some(Arc::new(RwLock::new(pixels)));

        let canvas = Canvas::new(
            self.pixels.as_ref().unwrap().clone(),
            self.width,
            self.height,
        );
        self.canvas = Some(canvas.clone());
        tokio::spawn(async move {
            ping_handler(canvas).await;
        });
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if event == WindowEvent::Destroyed && self.window_id == Some(window_id) {
            log::info!("window {:?} destroyed", window_id);
            self.window_id = None;
            event_loop.exit();
            return;
        }

        let window = match self.window.as_mut() {
            Some(window) => window,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                log::debug!("window {:?} closed", window.id());
                self.window = None;
            }
            WindowEvent::RedrawRequested => {
                self.canvas.as_mut().unwrap().set_queue_pixels();
                if let Err(err) = self.pixels.as_ref().unwrap().read().render() {
                    error!("pixels.render: {}", err);
                    event_loop.exit();
                }
            }
            _ => (),
        }
    }
}
