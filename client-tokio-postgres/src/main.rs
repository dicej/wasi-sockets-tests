#![deny(warnings)]

use {
    anyhow::{anyhow, Result},
    std::{env, net::SocketAddr, str::FromStr},
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let address = SocketAddr::from_str(
        &env::args()
            .nth(1)
            .ok_or_else(|| anyhow!("expected IPv4 or IPv6 socket address CLI argument"))?,
    )?;

    let (client, connection) = tokio_postgres::Config::new()
        .hostaddr(address.ip())
        .port(address.port())
        .user("test")
        .password("test")
        .connect(tokio_postgres::NoTls)
        .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    let rows = client.query("SELECT $1::TEXT", &[&"hello world"]).await?;

    assert_eq!(rows[0].get::<_, &str>(0), "hello world");

    Ok(())
}
