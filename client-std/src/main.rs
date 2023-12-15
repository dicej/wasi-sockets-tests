#![deny(warnings)]

use {
    anyhow::{anyhow, Context, Result},
    std::{
        env,
        io::{Read, Write},
        net::{SocketAddr, TcpStream, ToSocketAddrs},
        str::FromStr,
    },
};

fn main() -> Result<()> {
    let address = &env::args().nth(1).ok_or_else(|| {
        anyhow!("expected IPv4 or IPv6 socket address or <hostname>:<port> as CLI argument")
    })?;

    let addresses = if let Ok(address) = SocketAddr::from_str(address) {
        vec![address]
    } else {
        address
            .to_socket_addrs()
            .with_context(|| format!("unable to resolve {address:?}"))?
            .collect::<Vec<_>>()
    };

    for address in addresses {
        if let Ok(mut stream) = TcpStream::connect(address) {
            let message = b"So rested he by the Tumtum tree";
            stream.write_all(message)?;

            let mut buffer = vec![0; message.len()];
            stream.read_exact(&mut buffer)?;

            assert_eq!(message.as_slice(), &buffer);

            return Ok(());
        }
    }

    Err(anyhow!("unable to connect to {address:?}"))
}
