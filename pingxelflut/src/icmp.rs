use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io::{self, ErrorKind, Read},
    net::SocketAddr,
};

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
    /// - `target`: The target address of the ping.
    /// - `identifier`: The identifier of the ping.
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
    /// Apart from the send action this has the additional effect of incrementing the sequence number of this packet.
    ///
    /// Returns the socket used for sending so that responses can be received.
    pub fn send(&mut self) -> Result<Socket, io::Error> {
        self.encode();
        let socket = if self.target.is_ipv4() {
            Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?
        } else {
            Socket::new(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?
        };

        socket.send_to(&self.packet, &self.target.into())?;

        self.current_sequence_number = self.current_sequence_number.wrapping_add(1);
        self.update_seq(self.current_sequence_number);

        Ok(socket)
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

/// Read ICMP packets from the specified socket, and return the first payload that matches a certain condition.
pub(crate) fn read_icmp_packets_until(
    socket: &mut Socket,
    condition: impl Fn(&[u8]) -> bool,
) -> Result<Vec<u8>, io::Error> {
    let mut last_packet = Vec::new();

    loop {
        let mut buffer = [0; 2048];
        let second_result = socket.read(&mut buffer);
        match second_result {
            Err(why) => match why.kind() {
                ErrorKind::WouldBlock => {}
                ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe => break,
                _ => return Err(why),
            },
            Ok(size) => {
                // FIXME: only works on IPv4
                last_packet.resize(size - 20, 0);
                last_packet.copy_from_slice(&buffer[20..size]);
                println!("packet {:?}", last_packet);
                if condition(&last_packet) {
                    break;
                }
            }
        }
    }
    Ok(last_packet)
}

/// Read the first ICMP packet that has the specified type at payload index 4.
pub(crate) fn read_first_icmp_packet_with_type(
    socket: &mut Socket,
    receive_type: u8,
) -> Result<Vec<u8>, io::Error> {
    Ok(read_icmp_packets_until(socket, |buffer| {
        buffer.starts_with(&[0, 0]) && buffer.get(8).is_some_and(|v| *v == receive_type)
    })?)
}
