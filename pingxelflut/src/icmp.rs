//! ICMP send and receive tooling.
//!
//! This module is only available in std environments.

use etherparse::{Icmpv6Slice, SlicedPacket, TransportSlice};
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io::{self, ErrorKind, Read},
    net::SocketAddr,
};

/// Includes both the real header (4 bytes) as well as the echo standard data (4 bytes).
pub const ICMP_HEADER_SIZE: usize = 8;
pub const IPV4_HEADER_SIZE: usize = 20;
pub const ECHO_REQUEST_V4: u8 = 8;
pub const ECHO_REQUEST_V6: u8 = 128;
pub const ECHO_REPLY_V4: u8 = 0;
pub const ECHO_REPLY_V6: u8 = 129;

/// The two kinds of echo packets, request and reply.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EchoDirection {
    Request,
    Reply,
}

/// An ICMP v4/v6 Echo Request packet.
/// Provides functionality to send out Echo Request messages (pings) and capture their response.
// TODO: ICMPv6 is not implemented yet.
pub struct Icmp {
    direction: EchoDirection,
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
    /// - `direction`: The echo direction, i.e. an echo request or reply.
    pub fn new(target: SocketAddr, identifier: u16, direction: EchoDirection) -> Self {
        Icmp {
            direction,
            identifier,
            target,
            packet: [0; ICMP_HEADER_SIZE].to_vec(),
            payload: Vec::new(),
            current_sequence_number: 0,
        }
    }

    /// Set this ICMP packet’s custom payload.
    /// The first four bytes of the Echo Request packet are semi-standard and not affected by this payload.
    pub fn set_payload(&mut self, payload: Vec<u8>) {
        self.payload = payload;
    }

    /// lowest priority DSCP
    const DSCP_LOW_PRIORITY: u32 = 8 << 2;

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
        if self.target.is_ipv4() {
            socket.set_tos(Self::DSCP_LOW_PRIORITY)?;
        } else {
            socket.set_tclass_v6(Self::DSCP_LOW_PRIORITY)?;
        }

        socket.send_to(&self.packet, &self.target.into())?;

        self.current_sequence_number = self.current_sequence_number.wrapping_add(1);
        self.update_seq(self.current_sequence_number);

        Ok(socket)
    }

    /// Encode this packet’s data.
    fn encode(&mut self) {
        self.packet.truncate(ICMP_HEADER_SIZE);
        self.packet[0] = match (self.target.is_ipv4(), self.direction) {
            (true, EchoDirection::Request) => ECHO_REQUEST_V4,
            (true, EchoDirection::Reply) => ECHO_REPLY_V4,
            (false, EchoDirection::Request) => ECHO_REQUEST_V6,
            (false, EchoDirection::Reply) => ECHO_REPLY_V6,
        };

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
                if socket.local_addr().unwrap().is_ipv4() {
                    let ip_packet = SlicedPacket::from_ip(&buffer[..size])
                        .map_err(|_| io::Error::other("unknown packet type"))?;
                    match ip_packet.transport {
                        Some(TransportSlice::Icmpv4(icmp)) => {
                            if condition(icmp.payload()) {
                                icmp.payload().clone_into(&mut last_packet);
                                break;
                            }
                        }
                        _ => continue,
                    }
                } else {
                    let icmp = Icmpv6Slice::from_slice(&buffer[..size])
                        .map_err(|_| io::Error::other("unknown packet type"))?;
                    if condition(icmp.payload()) {
                        icmp.payload().clone_into(&mut last_packet);
                        break;
                    }
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
    // FIXME: use etherparse to more robustly read the packet type.
    read_icmp_packets_until(socket, |buffer| {
        buffer.first().is_some_and(|v| *v == receive_type)
    })
}

/// There’s no working async raw socket implementation for Rust at the moment, and I don’t want to implement a “real” one just for this.
/// Instead, run blocking reads on an additional thread and forward data through an async channel to the async workers.
pub struct IcmpListener {
    socket: Socket,
    send_queue: async_channel::Sender<(Vec<u8>, SocketAddr)>,
    pub receive_queue: async_channel::Receiver<(Vec<u8>, SocketAddr)>,
}

impl IcmpListener {
    pub fn new(is_ipv4: bool) -> Result<IcmpListener, io::Error> {
        let socket = if is_ipv4 {
            Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))?
        } else {
            Socket::new(Domain::IPV6, Type::RAW, Some(Protocol::ICMPV6))?
        };
        // set low priority
        if is_ipv4 {
            socket.set_tos(Icmp::DSCP_LOW_PRIORITY)?;
        } else {
            socket.set_tclass_v6(Icmp::DSCP_LOW_PRIORITY)?;
        }
        socket.set_nonblocking(false)?;
        Ok(Self::new_from_socket(socket))
    }

    pub fn new_from_socket(socket: Socket) -> Self {
        let (send_queue, receive_queue) = async_channel::unbounded();
        Self {
            socket,
            send_queue,
            receive_queue,
        }
    }

    /// Reads data from the socket in an infinite loop.
    pub fn run(&mut self) {
        let mut buffer = [0; 2048];
        loop {
            let result = self
                .socket
                .recv_from(unsafe { std::mem::transmute(buffer.as_mut_slice()) });
            match result {
                Err(why) => match why.kind() {
                    // socket closed, time to stop
                    ErrorKind::UnexpectedEof | ErrorKind::BrokenPipe => {
                        return;
                    }
                    _ => {}
                },
                Ok((size, address)) => {
                    let received_data = buffer[..size].to_owned();
                    let send_result = self.send_queue.send_blocking((
                        received_data,
                        address.as_socket().expect("only ip sockets are supported"),
                    ));
                    if send_result.is_err() {
                        return;
                    }
                }
            }
        }
    }
}
