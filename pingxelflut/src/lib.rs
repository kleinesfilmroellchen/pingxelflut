//! Common backend of the Pingxelflut client and server.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod format;
#[cfg(feature = "std")]
pub mod icmp;

#[cfg(feature = "std")]
mod std_functions {
    use std::io;
    use std::net::IpAddr;
    use std::net::SocketAddr;

    use crate::format::Color;
    use crate::format::Packet;
    use crate::icmp::read_first_icmp_packet_with_type;
    use crate::icmp::EchoDirection;
    use crate::icmp::Icmp;

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
        let response = Packet::from_bytes(&raw_response);
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
        set_request.send()?;
        Ok(())
    }
}

#[cfg(feature = "std")]
pub use std_functions::*;
