use socket2::{Domain, Protocol, Socket, Type};
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/**
 * Automatically binds to the passed address, depending on what it is.
 * If the passed addr is multicast, it is joined.
 * If the passed addr is not multicast, it is bound.
 *
 * All bound sockets are reusable.
 */
pub fn listen_udp(addr: &SocketAddr) -> Result<std::net::UdpSocket, io::Error> {
    if addr.ip().is_multicast() {
        join_multicast(addr)
    } else {
        bind_udp(addr)
    }
}

/**
 * Binds a non-multicast UDP address, with reuseaddr set.
 */
pub fn bind_udp(addr: &SocketAddr) -> Result<std::net::UdpSocket, io::Error> {
    let socket =
        match addr.ip() {
            IpAddr::V4(ref _mdns_v4) => Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
                .expect("ipv4 dgram socket"),

            IpAddr::V6(ref _mdns_v6) => Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
                .expect("ipv6 dgram socket"),
        };

    socket.set_nonblocking(true).expect("nonblocking Error");
    socket.set_reuse_address(true).expect("reuse addr Error");

    socket
        .bind(&socket2::SockAddr::from(addr.clone()))
        .expect("bind error");

    let std_sock: std::net::UdpSocket = socket.into();
    Ok(std_sock)
}

/// Returns a socket joined to the multicast address
pub fn join_multicast(multicast_addr: &SocketAddr) -> Result<std::net::UdpSocket, io::Error> {
    let ip_addr = multicast_addr.ip();
    if !ip_addr.is_multicast() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("expected multicast address for binding: {}", ip_addr),
        ));
    }

    let socket = match ip_addr {
        IpAddr::V4(ref mdns_v4) => {
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
                .expect("ipv4 dgram socket");
            socket
                .join_multicast_v4(mdns_v4, &Ipv4Addr::new(0, 0, 0, 0))
                .expect("join_multicast_v4");
            socket
        }
        IpAddr::V6(ref mdns_v6) => {
            let socket = Socket::new(Domain::IPV6, Type::DGRAM, Some(Protocol::UDP))
                .expect("ipv6 dgram socket");

            socket.set_only_v6(true)?;
            socket
                .join_multicast_v6(mdns_v6, 0)
                .expect("join_multicast_v6");
            socket
        }
    };

    socket.set_nonblocking(true).expect("nonblocking Error");
    socket.set_reuse_address(true).expect("reuse addr Error");
    #[cfg(unix)] // this is currently restricted to Unix's in socket2
    socket.set_reuse_port(true).expect("reuse port Error");
    bind_multicast(&socket, &multicast_addr).expect("bind Error");

    let udp: std::net::UdpSocket = socket.into();
    Ok(udp)
}

#[cfg(windows)]
fn bind_multicast(socket: &Socket, addr: &SocketAddr) -> io::Result<()> {
    let addr = match *addr {
        SocketAddr::V4(addr) => SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), addr.port()),
        SocketAddr::V6(addr) => {
            SocketAddr::new(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0).into(), addr.port())
        }
    };
    socket.bind(&socket2::SockAddr::from(addr))
}

#[cfg(unix)]
pub fn bind_multicast(socket: &Socket, addr: &SocketAddr) -> io::Result<()> {
    socket.bind(&socket2::SockAddr::from(*addr))
}
