//! Common backend of the Pingxelflut client and server.

use std::io;
use std::io::Write;
use std::net::IpAddr;
use std::net::SocketAddr;

use format::Color;
use format::Packet;
use icmp::read_first_icmp_packet_with_type;
use icmp::EchoDirection;
use icmp::Icmp;

pub mod format;
pub mod icmp;

/// Query and return the size of the provided Pingxelflut server.
pub fn get_size(target: IpAddr) -> Result<(u16, u16), io::Error> {
    let mut size_request = Icmp::new(
        SocketAddr::new(target, 0).to_owned(),
        0,
        EchoDirection::Request,
    );
    size_request.set_payload(Packet::SizeRequest.to_bytes());
    let mut socket = size_request.send()?;
    let raw_response = read_first_icmp_packet_with_type(&mut socket, Packet::SIZE_RESPONSE_ID)?;
    let response = Packet::from_bytes(&raw_response[8..]);
    match response {
        Some(Packet::SizeResponse { width, height }) => Ok((width, height)),
        Some(Packet::SizeRequest) => Err(io::Error::other("size request returned verbatim")),
        _ => Err(io::Error::other("invalid packet")),
    }
}

/// Set a single pixel on a target Pingxelflut server.
pub fn set_pixel(target: IpAddr, x: u16, y: u16, color: Color) -> Result<(), io::Error> {
    let mut set_request = Icmp::new(
        SocketAddr::new(target, 0).to_owned(),
        1,
        EchoDirection::Request,
    );
    set_request.set_payload(Packet::SetPixel { x, y, color }.to_bytes());
    let mut socket = set_request.send()?;
    socket.flush()?;
    Ok(())
}
