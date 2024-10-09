# `wasi-sockets` test harness

This is a test harness for prototyping support for
[wasi-sockets](https://github.com/WebAssembly/wasi-sockets) in
[tokio](https://tokio.rs/) and [CPython](https://github.com/python/cpython).

## Directory structure

- [server](./server): test host using
  [wasmtime](https://github.com/bytecodealliance/wasmtime) and
  [wasmtime-wasi](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi)
  to provide a host environment
- [client](./client): Rust test guest using `wasi-sockets` host functions
  directly
- [client-std](./client-std): Rust test guest using `std::net`.
- [client-tokio](./client-tokio): Rust test guest using `tokio::net`.
- [client-tokio-postgres](./client-tokio-postgres): Rust test guest using
  `tokio-postgres` on top of `tokio::net`.
- [client-python](./client-python): Python test using `asyncio`, built as a
  component by
  [componentize-py](https://github.com/bytecodealliance/componentize-py)
- [client-python-redis](./client-python-redis): Python test using `redis-py`

## Building and running

### Prerequisites

- Unix-style host (e.g. Linux, MacOS, Mingw, WSL2)
- Python
- Rust (with the `wasm32-wasip2` and `wasm32-unknown-unknown` targets installed)
    - As of this writing, the `wasm32-wasip2` target is only available in Rust
      nightly

In order to build `componentize-py`, we need to use a temporary fork of
`wasi-sdk`.  In the commands that follow, replace with your host platform's
target triple, and replace `macos` with `linux` or `mingw` (Windows) as
appropriate.  Note that if you're using e.g. Linux/ARM64, you'll need to build
the `wasi-sockets-alpha-5` branch of https://github.com/dicej/wasi-sdk from
source since there are not yet any pre-built distributions for that platform.

```shell
curl -LO https://github.com/dicej/wasi-sdk/releases/download/wasi-sockets-alpha-5/wasi-sdk-20.46gf3a1f8991535-macos.tar.gz
tar xf wasi-sdk-20.46gf3a1f8991535-macos.tar.gz
export WASI_SDK_PATH=$(pwd)/wasi-sdk-20.46gf3a1f8991535
```

Also, as of this writing, the `wasm-component-ld` binary shipped with Rust
nightly for the `wasm32-wasip2` target has a bug, so you'll need to upgrade it
manually, e.g. `cargo install wasm-component-ld --force --version 0.5.10 --root
$HOME/.rustup/toolchains/nightly-aarch64-unknown-linux-gnu/lib/rustlib/aarch64-unknown-linux-gnu`,
adjusting the path according to where the bin directory containing
`wasm-component-ld` is found.

Once the above is complete, you can switch to the `server` directory in your
clone of this repo and run the tests:

```shell
cd server
cargo test --release
```

All tests should pass.  If they don't, please open an issue on this repo.
