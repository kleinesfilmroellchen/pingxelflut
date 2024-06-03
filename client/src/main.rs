use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

use pingxelflut::Icmp;

fn main() {
    let mut echo = Icmp::new(
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0).to_owned(),
        28,
    );
    echo.set_payload(vec![1, 2, 3, 4]);
    println!("{:?}", echo.send().unwrap());
}
