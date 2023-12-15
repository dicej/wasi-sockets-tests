#![deny(warnings)]

use {
    anyhow::{anyhow, Context, Result},
    std::{
        env,
        net::{SocketAddr, ToSocketAddrs},
        str::FromStr,
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
        if let Ok((client, connection)) = tokio_postgres::Config::new()
            .hostaddr(address.ip())
            .port(address.port())
            .user("test")
            .password("test")
            .connect(tokio_postgres::NoTls)
            .await
        {
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("connection error: {e}");
                }
            });

            let rows = client.query("SELECT $1::TEXT", &[&"hello world"]).await?;

            assert_eq!(rows[0].get::<_, &str>(0), "hello world");

            return Ok(());
        }
    }

    Err(anyhow!("unable to connect to {address:?}"))
}
