#![deny(warnings)]

use {
    anyhow::{anyhow, Result},
    std::{
        env,
        io::{Read, Write},
        net::{SocketAddr, SocketAddrV4, SocketAddrV6, TcpStream},
        str::FromStr,
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

    let mut stream = TcpStream::connect(address)?;

    let message = b"So rested he by the Tumtum tree";
    stream.write_all(message)?;

    let mut buffer = vec![0; message.len()];
    stream.read_exact(&mut buffer)?;

    assert_eq!(message.as_slice(), &buffer);

    Ok(())
}
