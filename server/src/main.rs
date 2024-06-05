#![deny(clippy::all)]
#![forbid(unsafe_code)]
#![allow(clippy::single_match)]

mod canvas;
mod window;

use std::{
    net::{IpAddr, SocketAddr},
    thread,
};

use anyhow::Result;
use canvas::{to_internal_color, Canvas};
use etherparse::{Icmpv4Type, Icmpv6Slice, Icmpv6Type, SlicedPacket, TransportSlice};
use futures::{Future, StreamExt};
use log::{error, warn};
use pingxelflut::{
    format::Packet,
    icmp::{EchoDirection, Icmp, IcmpListener},
};
use window::App;
use winit::event_loop::EventLoop;

const WIDTH: u16 = 1920;
const HEIGHT: u16 = 1080;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new(WIDTH, HEIGHT);
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn decode_pingxelflut_packet(
    raw_packet: Vec<u8>,
    address: SocketAddr,
    is_ipv4: bool,
) -> Option<(Packet, IpAddr)> {
    // For some reason, under IPv4 we get an IP packet, while under IPv6 we get the ICMPv6 packet directly.
    // FIXME: this means we don’t know who sent the IPv6 packet! We just send the response to localhost.
    let transport_packet = if is_ipv4 {
        let parsed_packet = SlicedPacket::from_ip(&raw_packet).ok()?;
        parsed_packet.transport?
    } else {
        let icmpv6 = Icmpv6Slice::from_slice(&raw_packet).ok()?;
        TransportSlice::Icmpv6(icmpv6)
    };

    match transport_packet {
        TransportSlice::Icmpv4(data) => {
            let payload = data.payload();
            let packet_type = data.icmp_type();
            match packet_type {
                Icmpv4Type::EchoRequest(_) => {
                    Packet::from_bytes(payload).map(|p| (p, address.ip()))
                }
                _ => None,
            }
        }
        TransportSlice::Icmpv6(data) => {
            let payload = data.payload();
            let packet_type = data.icmp_type();
            match packet_type {
                Icmpv6Type::EchoRequest(_) => {
                    Packet::from_bytes(payload).map(|p| (p, address.ip()))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

async fn ip_ping_handler(canvas: Canvas, is_ipv4: bool) -> Result<()> {
    let mut icmp4_listener = IcmpListener::new(is_ipv4)?;
    let receive_queue = icmp4_listener.receive_queue.clone();

    thread::spawn(move || icmp4_listener.run());

    let stream = receive_queue.filter_map(|(data, addr)| {
        futures::future::ready(decode_pingxelflut_packet(data, addr, is_ipv4))
    });

    stream
        .for_each(move |(packet, target_addr)| {
            let mut canvas = canvas.clone();
            tokio::spawn(async move {
                match packet {
                    Packet::SizeRequest => {
                        // TODO: Figure out if the identifier is important for getting the packet delivered.
                        let mut response =
                            Icmp::new(SocketAddr::new(target_addr, 0), 0, EchoDirection::Reply);
                        response.set_payload(
                            Packet::SizeResponse {
                                width: WIDTH,
                                height: HEIGHT,
                            }
                            .to_bytes(),
                        );
                        let result = response.send();
                        match result {
                            Ok(_) => {}
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
            });
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
    futures::future::join(
        handle_error(ip_ping_handler(canvas.clone(), true)),
        handle_error(ip_ping_handler(canvas, false)),
    )
    .await;
}
