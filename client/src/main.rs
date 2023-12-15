#![deny(warnings)]

wit_bindgen::generate!("reactor" in "wit");

use {
    anyhow::{anyhow, Context, Result},
    std::{env, net::SocketAddr, str::FromStr},
    wasi::{
        io::streams::{InputStream, OutputStream},
        sockets::{
            instance_network, ip_name_lookup,
            network::{
                ErrorCode, IpAddress, IpAddressFamily, IpSocketAddress, Ipv4SocketAddress,
                Ipv6SocketAddress, Network,
            },
            tcp::TcpSocket,
            tcp_create_socket,
        },
    },
};

fn resolve(network: &Network, address: &str) -> Result<Vec<IpSocketAddress>> {
    Ok(if let Ok(address) = SocketAddr::from_str(&address) {
        vec![match address {
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
        }]
    } else {
        let (hostname, port) = address
            .split_once(':')
            .and_then(|(h, p)| u16::from_str(p).ok().map(|p| (h, p)))
            .ok_or_else(|| anyhow!("unable to parse {address} as <hostname>:<port>"))?;

        let context = || format!("unable to resolve {hostname:?}");

        let map = |address| match address {
            IpAddress::Ipv6(address) => IpSocketAddress::Ipv6(Ipv6SocketAddress {
                address,
                port,
                flow_info: 0,
                scope_id: 0,
            }),
            IpAddress::Ipv4(address) => IpSocketAddress::Ipv4(Ipv4SocketAddress { address, port }),
        };

        let stream = ip_name_lookup::resolve_addresses(&network, hostname).with_context(context)?;
        let mut addresses = Vec::new();
        loop {
            match stream.resolve_next_address() {
                Ok(Some(address)) => addresses.push(map(address)),
                Ok(None) => break addresses,
                Err(ErrorCode::WouldBlock) => stream.subscribe().block(),
                Err(error) => return Err(anyhow!(error)).with_context(context),
            }
        }
    })
}

fn connect(
    network: &Network,
    address: IpSocketAddress,
) -> Result<(TcpSocket, (InputStream, OutputStream))> {
    let client = tcp_create_socket::create_tcp_socket(match address {
        IpSocketAddress::Ipv6(_) => IpAddressFamily::Ipv6,
        IpSocketAddress::Ipv4(_) => IpAddressFamily::Ipv4,
    })?;

    client.start_connect(&network, address)?;
    Ok(loop {
        match client.finish_connect() {
            Err(ErrorCode::WouldBlock) => client.subscribe().block(),
            result => break (client, result?),
        }
    })
}

fn main() -> Result<()> {
    let address = &env::args().nth(1).ok_or_else(|| {
        anyhow!("expected IPv4 or IPv6 socket address or <hostname>:<port> as CLI argument")
    })?;

    let network = instance_network::instance_network();
    for address in resolve(&network, address)? {
        if let Ok((_client, (rx, tx))) = connect(&network, address) {
            let message = b"So rested he by the Tumtum tree";
            tx.blocking_write_and_flush(message)?;

            let mut buffer = Vec::with_capacity(message.len());
            while buffer.len() < message.len() {
                buffer
                    .extend(rx.blocking_read((message.len() - buffer.len()).try_into().unwrap())?);
            }

            assert_eq!(message.as_slice(), &buffer);

            drop((rx, tx));

            return Ok(());
        }
    }

    Err(anyhow!("unable to connect to {address:?}"))
}
