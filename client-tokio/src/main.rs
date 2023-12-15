#![deny(warnings)]

use {
    anyhow::{anyhow, Context, Result},
    std::{
        env,
        net::{SocketAddr, ToSocketAddrs},
        str::FromStr,
    },
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    },
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let address = &env::args().nth(1).ok_or_else(|| {
        anyhow!("expected IPv4 or IPv6 socket address or <hostname>:<port> as CLI argument")
    })?;

    let addresses = if let Ok(address) = SocketAddr::from_str(address) {
        vec![address]
    } else {
        // `tokio::net::lookup_host` won't work here since it needs to spawn a thread that calls `getaddrinfo`.  In
        // the future, we could patch `tokio` to use `wasi:sockets/ip-name-lookup` directly instead of going
        // through `getaddrinfo`, which would allow it to be async without multithreading.
        address
            .to_socket_addrs()
            .with_context(|| format!("unable to resolve {address:?}"))?
            .collect::<Vec<_>>()
    };

    for address in addresses {
        if let Ok(mut stream) = TcpStream::connect(address).await {
            let message = b"So rested he by the Tumtum tree";
            stream.write_all(message).await?;

            let mut buffer = vec![0; message.len()];
            stream.read_exact(&mut buffer).await?;

            assert_eq!(message.as_slice(), &buffer);

            return Ok(());
        }
    }

    Err(anyhow!("unable to connect to {address:?}"))
}
