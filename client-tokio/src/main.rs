#![deny(warnings)]

use {
    anyhow::{anyhow, Result},
    std::{
        env,
        net::{SocketAddr, SocketAddrV4, SocketAddrV6},
        str::FromStr,
    },
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    },
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let address = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("expected IPv4 or IPv6 socket address CLI argument"))?;

    let address = if let Ok(address) = SocketAddrV6::from_str(&address) {
        SocketAddr::V6(address)
    } else {
        SocketAddr::V4(SocketAddrV4::from_str(&address)?)
    };

    let mut stream = TcpStream::connect(address).await?;

    let message = b"So rested he by the Tumtum tree";
    stream.write_all(message).await?;

    let mut buffer = vec![0; message.len()];
    stream.read_exact(&mut buffer).await?;

    assert_eq!(message.as_slice(), &buffer);

    Ok(())
}
