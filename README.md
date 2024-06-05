# pingxelflut

Pixelflut, but with ICMP.

## Reference implementation structure

The reference implementation is split up into three Rust crates:

- `pingxelflut`: Common data structures and utilities for writing Rust pingxelflut implementations. May be published to crates.io at some point.
- `client`: Simple client implementation.
- `server`: Reasonably performant server implementation.

### Development and Usage

#### `client`

The client has a few options controlling how and where to send images, see its `--help` output. It needs to be able to open raw sockets, which requires the `cap_net_raw` capability on Linux. (Alternatively, run it as root.)

> ![WARNING]
> Currently, the client does not properly work on Windows: **It crashes your system**. The root cause of this issue is not know, since the client can seemingly send packets over raw sockets just fine. Additionally, it cannot receive more than one echo reply, meaning that requesting the canvas size does not work.

### `server`

The server does not have any options currently. It opens a window displaying the pingxelflut canvas; closing the window ends the application. It uses `libpcap` to detect ICMP packets, so the corresponding libraries must be installed; refer to your package manager of choice or install `Npcap` on Windows. The server needs the raw socket capabilities in addition to pcap permissions, so `cap_net_raw,cap_net_admin` seems to be required for Linux capabilities. (It doesn’t seem to be possible to run the server as root due to it interacting with the windowing system.)

> ![NOTE]
> The server is not tested on Windows.

For development, this command chain seems to be useful:

```shell
# in the `server` directory
cargo build --release && sudo setcap cap_net_raw,cap_net_admin=eip ../target/release/server && ../target/release/server
```

## Known Implementations

Please open a PR to add your implementation!

## Protocol

[RFC 2119 keywords](https://www.rfc-editor.org/rfc/rfc2119) are used in the following and MUST be interpreted accordingly.

The pingxelflut protocol is a layer 5 protocol based on [ICMP (RFC 792)](https://www.rfc-editor.org/rfc/rfc792) and any implementation MUST follow ICMP protocol requirements.

Messages to the server are sent as Echo Request packets (ICMP type 8 code 0, ICMPv6 type 128), messages to the client are sent as Echo Reply (ICMP type 0 code 0, ICMPv6 type 129).

The first four bytes of the payload are to be used according to Echo conventions. The first 16-bit word specifies the Echo request identifier, and the second 16-bit word specifies the Echo request sequence number. The identifier MUST be ignored. The sequence number of consecutive packets SHOULD be increasing.

The fifth byte of the payload specifies the packet type.

| Byte | Type          | Direction |
| ---- | ------------- | --------- |
| aa   | Size request  | To Server |
| bb   | Size response | To Client |
| cc   | Set pixel     | To Server |

All multi-byte values are in network order (big endian). (Since the color bytes are defined individually below, their byte order is RGB(A) and not BGR or else.)

Byte numbers in the following refer to the byte indices after the packet type byte.

### Size request

The size request packet contains no further data. The server responds with a size response packet. Size request packets MAY be rate-limited.

### Size response

The size response packet contains the server’s canvas size as two unsigned 16-bit integers.

| Bytes | Value  |
| ----- | ------ |
| 0-1   | Width  |
| 2-3   | Height |

### Set pixel

The set pixel packet contains an X and Y position to set a pixel at, plus an RGB(A) color to set. The coordinates are unsigned 16-bit integers, and the origin is in the top left corner of the image. The alpha value is optional.

| Bytes | Value            |
| ----- | ---------------- |
| 0-1   | X position       |
| 2-3   | Y position       |
| 4     | Red              |
| 5     | Green            |
| 6     | Blue             |
| 7     | Alpha (optional) |

The set pixel packet has no response.

### Invalid data handling recommendations

- Servers SHOULD silently discard pixel setting requests that fall outside the defined canvas. They MAY wrap pixel setting requests at the image borders (`x mod width` and `y mod height`).
- Since many systems can’t prevent default responses from ICMP Echo Request packets, any packet type that is invalid for its direction MUST be discarded by either side and not treated as an error.
- Any other kind of generally malformatted data MUST be discarded silently. Clients SHOULD warn the user about such events, for example to aid in debugging server implementations and raising issues with servers run at large events.

### Practical considerations

- Some network stacks may be ill-equipped to handle large amounts of ICMP packets. The Windows network stack has in testing shown to be one such example. Extra care needs to be taken when using such systems as part of a network that handles pingxelflut traffic.
- ICMP has no congestion control. Since clients can’t automatically decrease their sending rate, it is therefore recommended to silently drop ICMP packets in routers when the bandwidth limit is reached.
- ICMP cannot address applications, and on most operating systems any application receiving ICMP packets will recieve all ICMP packets sent to its machine (or at least to a specific link). While this does not limit the protocol itself (the only response message applies to all clients equally and may be read by anyone, even those that did not request it), it is therefore challenging to either run multiple clients on one machine, or to run a client on the same machine as a server. Additionally, running multiple distinct servers on one machine under one target IP address is not possible, but running a subordinate server that passively reads out pixel commands targeted at a main server may be useful.
