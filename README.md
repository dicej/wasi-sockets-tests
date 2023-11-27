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
  Rust, `wasi-libc`, and the WASI Preview 1 adapter as described below.

## Obtaining, building, and using the Rust, `wasi-libc`, and adapter forks

### Prerequisites

- Unix-style host (e.g. Linux, MacOS, Mingw, WSL2)
- CMake
- Clang
- Ninja
- Python
- Rust (with the `wasm32-wasi` and `wasm32-unknown-unknown` targets installed)

In the commands that follow, replace `aarch64-apple-darwin` with your host
platform's target triple.

Note that cloning the `llvm-project` submodule of `wasi-sdk` and the `rust` repo
may both take a _long_ time.

TODO: The following will build LLVM twice, which is annoying.  Can we avoid
that?

TODO #2: Can we speed up the Rust build by excluding tools we don't need?

```shell
git clone https://github.com/dicej/wasi-sdk -b sockets
cd wasi-sdk
git submodule update --init --recursive
make build/wasi-libc.BUILT
export WASI_SDK_SYSROOT=$(pwd)/build/install/opt/wasi-sdk/share/wasi-sysroot
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
git clone https://github.com/dicej/wasmtime -b adapter-open-badfd
cd wasmtime
git submodule update --init --recursive
bash ci/build-wasi-preview1-component-adapter.sh
export WASI_SOCKETS_TESTS_ADAPTER=$(pwd)/target/wasm32-unknown-unknown/release/wasi_snapshot_preview1.command.wasm
cd ..
```

Once the above is complete, you can switch to the `server` directory in your
clone of this repo and run the tests:

```shell
cd server
cargo test
```

All tests should pass.  If they don't, please open an issue on this repo.
