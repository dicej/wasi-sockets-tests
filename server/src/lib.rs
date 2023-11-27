#![deny(warnings)]

use {
    anyhow::{Context, Error, Result},
    futures::FutureExt,
    std::{future::Future, net::SocketAddr},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task,
    },
    tracing::log,
};

pub async fn serve(address: SocketAddr) -> Result<(impl Future<Output = Result<()>>, SocketAddr)> {
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("Unable to listen on {address}"))?;

    let address = listener.local_addr()?;

    Ok((
        async move {
            loop {
                let (mut stream, _) = listener.accept().await?;

                task::spawn(
                    async move {
                        let mut buffer = vec![0; 1024];
                        loop {
                            let count = stream.read(&mut buffer).await?;
                            if count == 0 {
                                break Ok::<_, Error>(());
                            }

                            stream.write_all(&buffer[..count]).await?;
                        }
                    }
                    .map(|result| {
                        if let Err(e) = result {
                            log::warn!("error handling connection: {e:?}");
                        }
                    }),
                );
            }
        }
        .boxed(),
        address,
    ))
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        anyhow::anyhow,
        futures::{channel::oneshot, future},
        std::{
            env,
            net::{Ipv4Addr, Ipv6Addr},
            sync::Once,
        },
        tokio::{fs, process::Command},
        wasmtime::{
            component::{Component, Linker},
            Config, Engine, Store,
        },
        wasmtime_wasi::preview2::{command, Table, WasiCtx, WasiCtxBuilder, WasiView},
        wit_component::ComponentEncoder,
    };

    struct SocketsCtx {
        table: Table,
        wasi: WasiCtx,
    }

    impl WasiView for SocketsCtx {
        fn table(&self) -> &Table {
            &self.table
        }
        fn table_mut(&mut self) -> &mut Table {
            &mut self.table
        }
        fn ctx(&self) -> &WasiCtx {
            &self.wasi
        }
        fn ctx_mut(&mut self) -> &mut WasiCtx {
            &mut self.wasi
        }
    }

    async fn build_component(src_path: &str, name: &str) -> Result<Vec<u8>> {
        let adapter_path = if let Some(path) = env::var("WASI_SOCKETS_TESTS_ADAPTER").ok() {
            path
        } else {
            let adapter_url = "https://github.com/bytecodealliance/wasmtime/releases\
                               /download/v14.0.4/wasi_snapshot_preview1.command.wasm";

            let adapter_path = "../target/wasi_snapshot_preview1.command.wasm";

            if !fs::try_exists(adapter_path).await? {
                fs::write(
                    adapter_path,
                    reqwest::get(adapter_url).await?.bytes().await?,
                )
                .await?;
            }
            adapter_path.to_owned()
        };

        let toolchain =
            env::var("WASI_SOCKETS_TESTS_TOOLCHAIN").unwrap_or_else(|_| "stable".to_owned());

        if Command::new("cargo")
            .current_dir(src_path)
            .args([
                format!("+{toolchain}").as_str(),
                "build",
                "--target",
                "wasm32-wasi",
            ])
            .status()
            .await?
            .success()
        {
            Ok(ComponentEncoder::default()
                .validate(true)
                .adapter("wasi_snapshot_preview1", &fs::read(adapter_path).await?)?
                .module(&fs::read(format!("../target/wasm32-wasi/debug/{name}.wasm")).await?)?
                .encode()?)
        } else {
            Err(anyhow!("cargo build failed"))
        }
    }

    async fn test(src_path: &str, name: &str, address: SocketAddr) -> Result<()> {
        static ONCE: Once = Once::new();
        ONCE.call_once(pretty_env_logger::init);

        let (server, address) = serve(address).await?;

        let (_tx, rx) = oneshot::channel::<()>();

        task::spawn(async move {
            drop(future::select(server, rx).await);
        });

        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

        let component = Component::new(&engine, build_component(src_path, name).await?)?;

        let mut linker = Linker::new(&engine);

        command::add_to_linker(&mut linker)?;

        let table = Table::new();
        let wasi = WasiCtxBuilder::new()
            .inherit_stdio()
            .inherit_network(wasmtime_wasi::ambient_authority())
            .arg("sockets-client")
            .arg(format!("{address}"))
            .build();

        let mut store = Store::new(&engine, SocketsCtx { table, wasi });

        let (command, _) =
            command::Command::instantiate_async(&mut store, &component, &linker).await?;

        command
            .wasi_cli_run()
            .call_run(&mut store)
            .await?
            .map_err(|()| anyhow::anyhow!("command returned with failing exit status"))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn direct_ipv4() -> Result<()> {
        test(
            "../client",
            "sockets-client",
            (Ipv4Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn direct_ipv6() -> Result<()> {
        test(
            "../client",
            "sockets-client",
            (Ipv6Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn std_ipv4() -> Result<()> {
        test(
            "../client-std",
            "sockets-client-std",
            (Ipv4Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn std_ipv6() -> Result<()> {
        test(
            "../client-std",
            "sockets-client-std",
            (Ipv6Addr::LOCALHOST, 0).into(),
        )
        .await
    }
}
