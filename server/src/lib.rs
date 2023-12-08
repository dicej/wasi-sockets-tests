#![deny(warnings)]

use {
    anyhow::{Context, Error, Result},
    async_trait::async_trait,
    futures::{stream, FutureExt},
    pgwire::{
        api::{
            auth::noop::NoopStartupHandler,
            portal::{Format, Portal},
            query::{ExtendedQueryHandler, SimpleQueryHandler, StatementOrPortal},
            results::{DataRowEncoder, DescribeResponse, FieldInfo, QueryResponse, Response},
            stmt::NoopQueryParser,
            store::MemPortalStore,
            ClientInfo, Type,
        },
        error::PgWireResult,
    },
    std::{future::Future, iter, net::SocketAddr, sync::Arc},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        task,
    },
    tracing::log,
};

pub async fn serve_echo(
    address: SocketAddr,
) -> Result<(impl Future<Output = Result<()>>, SocketAddr)> {
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

#[derive(Default)]
struct MyQueryHandler {
    portal_store: Arc<MemPortalStore<String>>,
    query_parser: Arc<NoopQueryParser>,
}

impl MyQueryHandler {
    fn schema(format: &Format) -> Vec<FieldInfo> {
        vec![FieldInfo::new(
            "text".into(),
            None,
            None,
            Type::TEXT,
            format.format_for(0),
        )]
    }
}

#[async_trait]
impl SimpleQueryHandler for MyQueryHandler {
    async fn do_query<'a, C>(&self, _client: &C, _query: &'a str) -> PgWireResult<Vec<Response<'a>>>
    where
        C: ClientInfo + Unpin + Send + Sync,
    {
        todo!()
    }
}

#[async_trait]
impl ExtendedQueryHandler for MyQueryHandler {
    type Statement = String;
    type QueryParser = NoopQueryParser;
    type PortalStore = MemPortalStore<Self::Statement>;

    fn portal_store(&self) -> Arc<Self::PortalStore> {
        self.portal_store.clone()
    }

    fn query_parser(&self) -> Arc<Self::QueryParser> {
        self.query_parser.clone()
    }

    async fn do_query<'a, C>(
        &self,
        _client: &mut C,
        portal: &'a Portal<Self::Statement>,
        _max_rows: usize,
    ) -> PgWireResult<Response<'a>>
    where
        C: ClientInfo + Unpin + Send + Sync,
    {
        let query = portal.statement().statement();
        assert_eq!("SELECT $1::TEXT", query.as_str());

        let parameters = portal.parameters();
        assert_eq!(1, parameters.len());
        assert_eq!(Some(b"hello world" as &[_]), parameters[0].as_deref());

        let schema = Arc::new(Self::schema(&Format::UnifiedText));
        let mut encoder = DataRowEncoder::new(schema.clone());
        encoder.encode_field(&"hello world")?;
        Ok(Response::Query(QueryResponse::new(
            schema,
            stream::iter(iter::once(encoder.finish())),
        )))
    }

    async fn do_describe<C>(
        &self,
        _client: &mut C,
        target: StatementOrPortal<'_, Self::Statement>,
    ) -> PgWireResult<DescribeResponse>
    where
        C: ClientInfo + Unpin + Send + Sync,
    {
        match target {
            StatementOrPortal::Statement(_) => Ok(DescribeResponse::new(
                Some(vec![Type::TEXT]),
                Self::schema(&Format::UnifiedText),
            )),
            StatementOrPortal::Portal(portal) => Ok(DescribeResponse::new(
                None,
                Self::schema(portal.result_column_format()),
            )),
        }
    }
}

pub async fn serve_postgres(
    address: SocketAddr,
) -> Result<(impl Future<Output = Result<()>>, SocketAddr)> {
    let listener = TcpListener::bind(address)
        .await
        .with_context(|| format!("Unable to listen on {address}"))?;

    let address = listener.local_addr()?;

    Ok((
        async move {
            loop {
                let (stream, _) = listener.accept().await?;

                task::spawn(
                    async move {
                        pgwire::tokio::process_socket(
                            stream,
                            None,
                            Arc::new(NoopStartupHandler),
                            Arc::new(MyQueryHandler::default()),
                            Arc::new(MyQueryHandler::default()),
                        )
                        .await
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
            path::Path,
            sync::Once,
        },
        tempfile::NamedTempFile,
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
        let adapter_path = if let Ok(path) = env::var("WASI_SOCKETS_TESTS_ADAPTER") {
            path
        } else {
            let adapter_url = "https://github.com/bytecodealliance/wasmtime/releases\
                               /download/v16.0.0/wasi_snapshot_preview1.command.wasm";

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
                .module(&fs::read(format!("../target/wasm32-wasi/debug/{name}.wasm")).await?)?
                .adapter("wasi_snapshot_preview1", &fs::read(adapter_path).await?)?
                .encode()?)
        } else {
            Err(anyhow!("cargo build failed"))
        }
    }

    async fn build_python_component(src_path: &str) -> Result<Vec<u8>> {
        let tmp = NamedTempFile::new()?;
        componentize_py::componentize(
            &Path::new("../client/wit"),
            Some("wasi:cli/command@0.2.0-rc-2023-12-05"),
            &[src_path],
            "app",
            tmp.path(),
            None,
        )
        .await?;
        Ok(fs::read(tmp.path()).await?)
    }

    async fn test_postgres(src_path: &str, name: &str, address: SocketAddr) -> Result<()> {
        test(&build_component(src_path, name).await?, async move {
            serve_postgres(address).await
        })
        .await
    }

    async fn test_echo(src_path: &str, name: &str, address: SocketAddr) -> Result<()> {
        test(&build_component(src_path, name).await?, async move {
            serve_echo(address).await
        })
        .await
    }

    async fn test_python_echo(src_path: &str, address: SocketAddr) -> Result<()> {
        test(&build_python_component(src_path).await?, async move {
            serve_echo(address).await
        })
        .await
    }

    async fn test(
        component: &[u8],
        serve: impl Future<
            Output = Result<(
                impl Future<Output = Result<()>> + Unpin + Send + 'static,
                SocketAddr,
            )>,
        >,
    ) -> Result<()> {
        static ONCE: Once = Once::new();
        ONCE.call_once(pretty_env_logger::init);

        let (server, address) = serve.await?;

        let (_tx, rx) = oneshot::channel::<()>();

        task::spawn(async move {
            drop(future::select(server, rx).await);
        });

        let mut config = Config::new();
        config.wasm_component_model(true);
        config.async_support(true);

        let engine = Engine::new(&config)?;

        let component = Component::new(&engine, &component)?;

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
        test_echo(
            "../client",
            "sockets-client",
            (Ipv4Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn direct_ipv6() -> Result<()> {
        test_echo(
            "../client",
            "sockets-client",
            (Ipv6Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn std_ipv4() -> Result<()> {
        test_echo(
            "../client-std",
            "sockets-client-std",
            (Ipv4Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn std_ipv6() -> Result<()> {
        test_echo(
            "../client-std",
            "sockets-client-std",
            (Ipv6Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tokio_ipv4() -> Result<()> {
        test_echo(
            "../client-tokio",
            "sockets-client-tokio",
            (Ipv4Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tokio_ipv6() -> Result<()> {
        test_echo(
            "../client-tokio",
            "sockets-client-tokio",
            (Ipv6Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn tokio_postgres() -> Result<()> {
        test_postgres(
            "../client-tokio-postgres",
            "sockets-client-tokio-postgres",
            (Ipv6Addr::LOCALHOST, 0).into(),
        )
        .await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn python_ipv4() -> Result<()> {
        test_python_echo("../client-python", (Ipv4Addr::LOCALHOST, 0).into()).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn python_ipv6() -> Result<()> {
        test_python_echo("../client-python", (Ipv6Addr::LOCALHOST, 0).into()).await
    }
}
