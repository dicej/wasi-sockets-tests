#![deny(warnings)]

wit_bindgen::generate!("preview1-adapter-reactor" in "wit");

use {
    anyhow::{anyhow, Result},
    std::{
        env,
        net::{SocketAddr, SocketAddrV4, SocketAddrV6},
        str::FromStr,
    },
    wasi::{
        io::poll,
        sockets::{
            instance_network,
            network::{
                ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress, Ipv6SocketAddress,
            },
            tcp_create_socket,
        },
    },
};

fn main() -> Result<()> {
    let address = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("expected ipv4 address CLI argument"))?;

    let address = if let Ok(address) = SocketAddrV6::from_str(&address) {
        SocketAddr::V6(address)
    } else {
        SocketAddr::V4(SocketAddrV4::from_str(&address)?)
    };

    let network = instance_network::instance_network();

    let client = tcp_create_socket::create_tcp_socket(match address {
        SocketAddr::V6(_) => IpAddressFamily::Ipv6,
        SocketAddr::V4(_) => IpAddressFamily::Ipv4,
    })?;

    client.start_connect(
        &network,
        match address {
            SocketAddr::V6(address) => {
                let ip = address.ip().segments();
                IpSocketAddress::Ipv6(Ipv6SocketAddress {
                    address: (ip[0], ip[1], ip[2], ip[3], ip[4], ip[5], ip[6], ip[7]),
                    port: address.port(),
                    flow_info: 0,
                    scope_id: 0,
                })
            }
            SocketAddr::V4(address) => {
                let ip = address.ip().octets();
                IpSocketAddress::Ipv4(Ipv4SocketAddress {
                    address: (ip[0], ip[1], ip[2], ip[3]),
                    port: address.port(),
                })
            }
        },
    )?;

    let (rx, tx) = loop {
        match client.finish_connect() {
            Err(ErrorCode::WouldBlock) => poll::poll_one(&client.subscribe()),
            result => break result,
        }
    }?;

    let message = b"So rested he by the Tumtum tree";
    tx.blocking_write_and_flush(message)?;

    let mut buffer = Vec::with_capacity(message.len());
    while buffer.len() < message.len() {
        buffer.extend(rx.blocking_read((message.len() - buffer.len()).try_into().unwrap())?);
    }

    assert_eq!(message.as_slice(), &buffer);

    Ok(())
}
