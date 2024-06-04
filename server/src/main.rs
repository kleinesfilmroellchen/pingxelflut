#![deny(clippy::all)]
#![forbid(unsafe_code)]
#![allow(clippy::single_match)]

mod canvas;

use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use anyhow::Result;
use canvas::{to_internal_color, Canvas};
use env_logger;
use etherparse::{Icmpv4Type, NetSlice, SlicedPacket, TransportSlice};
use futures::{Future, StreamExt};
use log::{error, warn};
use parking_lot::RwLock;
use pcap::{Capture, Device, PacketCodec};
use pingxelflut::format::Packet;
use pingxelflut::icmp::{EchoDirection, Icmp};
use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 240;

#[derive(Default)]
struct App {
    window_id: Option<WindowId>,
    window: Option<Arc<Window>>,
    pixels: Option<Arc<RwLock<Pixels>>>,
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
            .with_inner_size(winit::dpi::LogicalSize::new(WIDTH, HEIGHT));

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        self.window_id = Some(window.id());
        self.window = Some(window.clone());

        let window = self.window.as_ref().unwrap().clone();
        let pixels = {
            let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, &window);
            Pixels::new(WIDTH, HEIGHT, surface_texture).unwrap()
        };
        self.pixels = Some(Arc::new(RwLock::new(pixels)));

        let canvas = Canvas {
            width: WIDTH as u16,
            height: HEIGHT as u16,
            pixels: self.pixels.as_ref().unwrap().clone(),
        };
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
                if let Err(err) = self.pixels.as_ref().unwrap().read().render() {
                    error!("pixels.render: {}", err);
                    event_loop.exit();
                    return;
                }
            }
            _ => (),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

struct PingxelflutPacketStream;

/// Extract the IP source address from a parsed network layer packet.
/// Works for both IP versions.
fn ip_addr_from_net_packet(packet: &NetSlice) -> IpAddr {
    match packet {
        NetSlice::Ipv4(ip_packet) => ip_packet.header().source_addr().into(),
        NetSlice::Ipv6(ip_packet) => ip_packet.header().source_addr().into(),
    }
}

impl PacketCodec for PingxelflutPacketStream {
    type Item = Option<(Packet, IpAddr)>;

    fn decode(&mut self, packet: pcap::Packet<'_>) -> Self::Item {
        let parsed_packet = SlicedPacket::from_ethernet(&packet).ok()?;
        let transport_packet = parsed_packet.transport?;
        let destination_address = ip_addr_from_net_packet(&parsed_packet.net?);

        match transport_packet {
            TransportSlice::Icmpv4(data) => {
                let payload = data.payload();
                let packet_type = data.icmp_type();
                match packet_type {
                    Icmpv4Type::EchoRequest(_) => {
                        Packet::from_bytes(payload).map(|p| (p, destination_address))
                    }
                    _ => None,
                }
            }
            TransportSlice::Icmpv6(_data) => {
                warn!("ICMPv6 not yet supported");
                None
            }
            _ => None,
        }
    }
}

async fn device_ping_handler(mut canvas: Canvas, device: Device) -> Result<()> {
    let mut capture = Capture::from_device(device)?
        .immediate_mode(true)
        .promisc(true)
        .snaplen(512)
        .open()?
        .setnonblock()?;

    // FIXME: support ICMPv6 by changing the filter to "icmp or icmp6"
    capture.filter("icmp", false)?;
    let stream = capture.stream(PingxelflutPacketStream)?;

    // let capture = Arc::new(Mutex::new(capture));

    stream
        .for_each_concurrent(None, move |maybe_packet| {
            if let Ok(Some((packet, target_addr))) = maybe_packet {
                match packet {
                    Packet::SizeRequest => {
                        // TODO: Figure out if the identifier is important for getting the packet delivered.
                        let mut response =
                            Icmp::new(SocketAddr::new(target_addr, 0), 0, EchoDirection::Reply);
                        let result = response.send();
                        match result {
                            Ok(mut socket) => {
                                let _ = socket.flush();
                            }
                            Err(why) => {
                                warn!("size response error: {}", why)
                            }
                        }
                    }
                    // ignore
                    Packet::SizeResponse { .. } => {}
                    Packet::SetPixel { x, y, color } => {
                        canvas.set_pixel(x, y, to_internal_color(color));
                    }
                }
            }
            // stream.capture_mut().sendpacket([1,2,3,4]);
            // capture.sendpacket([1,2,3,4]);
            futures::future::ready(())
        })
        .await;
    Ok(())
}

/// Handle an error, but ignore it.
async fn handle_error(future: impl Future<Output = Result<()>>) {
    let result = future.await;
    match result {
        Err(why) => {
            error!("error in async task: {}", why);
        }
        Ok(_) => {}
    }
}

async fn ping_handler(canvas: Canvas) {
    let devices = Device::list().unwrap();
    let device_iter = futures::stream::iter(devices.into_iter());
    device_iter
        .for_each_concurrent(None, |device| {
            handle_error(device_ping_handler(canvas.clone(), device))
        })
        .await;
}
