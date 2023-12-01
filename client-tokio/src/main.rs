#![deny(warnings)]

use {
    anyhow::{anyhow, Result},
    std::{env, net::SocketAddr, str::FromStr},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    },
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let address = SocketAddr::from_str(
        &env::args()
            .nth(1)
            .ok_or_else(|| anyhow!("expected IPv4 or IPv6 socket address CLI argument"))?,
    )?;

    let mut stream = TcpStream::connect(address).await?;

    let message = b"So rested he by the Tumtum tree";
    stream.write_all(message).await?;

    let mut buffer = vec![0; message.len()];
    stream.read_exact(&mut buffer).await?;

    assert_eq!(message.as_slice(), &buffer);

    Ok(())
}
