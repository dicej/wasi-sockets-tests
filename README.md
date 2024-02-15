# `wasi-sockets` test harness

This is a test harness for prototyping support for
[wasi-sockets](https://github.com/WebAssembly/wasi-sockets) in the Rust `std`
library and `wasi-libc`.

Note that you can already use `wasi-sockets` by calling host functions directly
from C, Rust, Python, and other languages with [Component
Model](https://github.com/WebAssembly/component-model) support.  What's missing
as of this writing is standard library support for those languages, which is
what this repository is intended to exercise as progress is made.

## Directory structure

- [server](./server): test host using
  [wasmtime](https://github.com/bytecodealliance/wasmtime) and
  [wasmtime-wasi](https://github.com/bytecodealliance/wasmtime/tree/main/crates/wasi)
  to provide a host environment
- [client](./client): Rust test guest using `wasi-sockets` host functions
  directly
- [client-std](./client-std): Rust test guest using `std::net`.  Note that, as
  of this writing, tests using this guest will fail unless you use forks of
  Rust and `wasi-libc` as described below.
- [client-tokio](./client-tokio): Rust test guest using `tokio::net`.  As with
  `client-std`, you'll need to use the forks discussed below.
- [client-tokio-postgres](./client-tokio-postgres): Rust test guest using
  `tokio-postgres` on top of `tokio::net`.  As with `client-std`, you'll need to
  use the forks discussed below.
- [client-python](./client-python): Python test using `asyncio`, built as a
  component by
  [componentize-py](https://github.com/bytecodealliance/componentize-py)

## Building and running

### Prerequisites

- Unix-style host (e.g. Linux, MacOS, Mingw, WSL2)
- Python
- Rust (with the `wasm32-wasi` and `wasm32-unknown-unknown` targets installed)

In the commands that follow, replace `aarch64-apple-darwin` with your host
platform's target triple, and replace `macos` with `linux` or `mingw` (Windows)
as appropriate.

Note that cloning the `llvm-project` submodule of the `rust` repo may both take
a _long_ time.

TODO: Can we speed up the Rust build by excluding tools we don't need?

```shell
curl -LO https://github.com/dicej/wasi-sdk/releases/download/wasi-sockets-alpha-2/wasi-sdk-20.26g68203b20b82e-macos.tar.gz
tar xf wasi-sdk-20.26g68203b20b82e-macos.tar.gz
export WASI_SDK_PATH=$(pwd)/wasi-sdk-20.26g68203b20b82e
export WASI_SDK_SYSROOT=$WASI_SDK_PATH/share/wasi-sysroot
cd ..
git clone https://github.com/dicej/rust -b sockets
cd rust
./configure \
    --target=wasm32-wasi,wasm32-unknown-unknown,aarch64-apple-darwin \
    --set=target.wasm32-wasi.wasi-root=$WASI_SDK_SYSROOT \
    --enable-lld
./x.py build --stage 1
rustup toolchain link wasi-sockets build/host/stage1
export WASI_SOCKETS_TESTS_TOOLCHAIN=wasi-sockets
cd ..
```

Once the above is complete, you can switch to the `server` directory in your
clone of this repo and run the tests:

```shell
cd server
cargo test --release
```

All tests should pass.  If they don't, please open an issue on this repo.
