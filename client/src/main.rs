#![deny(warnings)]

wit_bindgen::generate!("preview1-adapter-reactor" in "wit");

use {
    anyhow::{anyhow, Result},
    std::{env, net::SocketAddrV4, str::FromStr},
    wasi::{
        io::poll,
        sockets::{
            instance_network,
            network::{ErrorCode, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress},
            tcp_create_socket,
        },
    },
};

fn main() -> Result<()> {
    let network = instance_network::instance_network();
    let client = tcp_create_socket::create_tcp_socket(IpAddressFamily::Ipv4)?;
    let pollable = client.subscribe();
    let address = SocketAddrV4::from_str(
        &env::args()
            .nth(1)
            .ok_or_else(|| anyhow!("expected ipv4 address CLI argument"))?,
    )?;
    let ip = address.ip().octets();
    client.start_connect(
        &network,
        IpSocketAddress::Ipv4(Ipv4SocketAddress {
            address: (ip[0], ip[1], ip[2], ip[3]),
            port: address.port(),
        }),
    )?;
    let (rx, tx) = loop {
        match client.finish_connect() {
            Err(ErrorCode::WouldBlock) => poll::poll_one(&pollable),
            result => break result,
        }
    }?;

    let message = b"So rested he by the Tumtum tree";
    tx.blocking_write_and_flush(message)?;

    let mut buffer = Vec::with_capacity(message.len());
    while buffer.len() < message.len() {
        buffer.extend(rx.read((message.len() - buffer.len()).try_into().unwrap())?);
    }

    assert_eq!(message.as_slice(), &buffer);

    Ok(())
}
