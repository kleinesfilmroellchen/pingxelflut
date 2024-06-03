//! Common backend of the Pingxelflut client and server.

use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io::{self, Read},
    net::SocketAddr,
};

pub mod format;

const HEADER_SIZE: usize = 8;
const ECHO_V4: u8 = 8;
const ECHO_V6: u8 = 128;

/// An ICMP v4/v6 Echo Request packet.
/// Provides functionality to send out Echo Request messages (pings) and capture their response.
// TODO: ICMPv6 is not implemented yet.
pub struct Icmp {
    /// Ping identifier, part of the standard payload.
    identifier: u16,
    /// Target address.
    target: SocketAddr,
    // Raw packet data, reused across subsequent packet sends for performance.
    packet: Vec<u8>,
    /// Non-standard payload to be sent.
    payload: Vec<u8>,
    /// Ping sequence number, part of the standard payload.
    current_sequence_number: u16,
}

impl Icmp {
    /// Create a new ICMP packet.
    ///
    /// - `target`: The target address of the ping
    pub fn new(target: SocketAddr, identifier: u16) -> Self {
        Icmp {
            identifier,
            target,
            packet: [0; HEADER_SIZE].to_vec(),
            payload: Vec::new(),
            current_sequence_number: 0,
        }
    }

    /// Set this ICMP packet’s custom payload.
    /// The first four bytes of the Echo Request packet are semi-standard and not affected by this payload.
    pub fn set_payload(&mut self, payload: Vec<u8>) {
        self.payload = payload;
    }

    /// Send this ICMP packet.
    /// Apart from the send action this has multiple additional effects:
    /// - Increment the sequence number of this packet.
    /// - Receive the first response packet and return the raw ICMP packet.
    pub fn send(&mut self) -> Result<Vec<u8>, io::Error> {
        self.encode();
        let mut socket = if self.target.is_ipv4() {
            Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?
        } else {
            Socket::new(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?
        };

        socket.send_to(&self.packet, &self.target.into())?;

        let mut buffer = [0; 2048];
        let size = socket.read(&mut buffer).unwrap();
        // FIXME: only works on IPv4
        let icmp_packet = &buffer[20 + 4..size];

        self.current_sequence_number = self.current_sequence_number.wrapping_add(1);
        self.update_seq(self.current_sequence_number);

        Ok(icmp_packet.to_owned())
    }

    /// Encode this packet’s data.
    fn encode(&mut self) {
        self.packet.truncate(8);
        if self.target.is_ipv4() {
            self.packet[0] = ECHO_V4;
        } else {
            self.packet[0] = ECHO_V6;
        }

        self.packet[1] = 0;
        self.packet[4] = (self.identifier >> 8) as u8;
        self.packet[5] = self.identifier as u8;
        self.packet[6] = 0;
        self.packet[7] = 0;
        self.packet.append(&mut self.payload.clone());
        self.checksum();
    }

    /// Update this packet’s sequence number.
    fn update_seq(&mut self, seq: u16) {
        self.packet[2] = 0;
        self.packet[3] = 0;
        self.packet[6] = (seq >> 8) as u8;
        self.packet[7] = seq as u8;
        self.checksum();
    }

    /// Update this packet’s checksum.
    fn checksum(&mut self) {
        let mut sum = 0u32;
        for word in self.packet.chunks(2) {
            let mut part = u16::from(word[0]) << 8;
            if word.len() > 1 {
                part += u16::from(word[1]);
            }
            sum = sum.wrapping_add(u32::from(part));
        }
        while (sum >> 16) > 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        let sum = !sum as u16;
        self.packet[2] = (sum >> 8) as u8;
        self.packet[3] = (sum & 0xff) as u8;
    }
}
