#![deny(warnings)]

wit_bindgen::generate!({
    world: "spin",
    exports: {
        "wasi:http/incoming-handler": Spin
    }
});

use {
    anyhow::Result,
    exports::wasi::http::incoming_handler::{Guest, IncomingRequest, ResponseOutparam},
    std::{net::SocketAddr, str::FromStr},
    wasi::{
        http::types::{Headers, OutgoingResponse},
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

struct Spin;

impl Guest for Spin {
    fn handle(_request: IncomingRequest, response_out: ResponseOutparam) {
        if let Err(e) = main() {
            eprintln!("FAIL: {e}");
        }
        ResponseOutparam::set(
            response_out,
            Ok(OutgoingResponse::new(200, &Headers::new(&[]))),
        );
    }
}

fn main() -> Result<()> {
    let address = SocketAddr::from_str("127.0.0.1:5001")?;

    let client = tcp_create_socket::create_tcp_socket(match address {
        SocketAddr::V6(_) => IpAddressFamily::Ipv6,
        SocketAddr::V4(_) => IpAddressFamily::Ipv4,
    })?;

    client.start_connect(
        &instance_network::instance_network(),
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

    println!("{:?}", std::str::from_utf8(&buffer));

    Ok(())
}
