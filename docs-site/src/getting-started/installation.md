# Installation

ThetaDataDx is available for Rust, Python, Go, and C++. Choose your language below.

## Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
thetadatadx = "2.0"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Or install from the repository:

```bash
git clone https://github.com/userFRM/ThetaDataDx.git
cd ThetaDataDx
cargo build --release
```

### Minimum Supported Rust Version

See `rust-toolchain.toml` in the repository root. The project requires a recent stable Rust toolchain with `rustfmt` and `clippy`.

## Python

```bash
pip install thetadatadx
```

With DataFrame support:

```bash
# pandas support
pip install thetadatadx[pandas]

# polars support
pip install thetadatadx[polars]

# both
pip install thetadatadx[all]
```

Requires Python 3.9+. The Python SDK is a native extension built with PyO3 -- no Rust toolchain required when installing from PyPI (pre-built wheels are provided).

### Building from Source

If you need to build the Python SDK from source (e.g., for an unsupported platform):

```bash
pip install maturin
git clone https://github.com/userFRM/ThetaDataDx.git
cd ThetaDataDx/sdks/python
maturin develop --release
```

## Go

First, build the Rust FFI library:

```bash
git clone https://github.com/userFRM/ThetaDataDx.git
cd ThetaDataDx
cargo build --release -p thetadatadx-ffi
```

This produces `target/release/libthetadatadx_ffi.so` (Linux) or `libthetadatadx_ffi.dylib` (macOS).

Then add the Go module:

```bash
go get github.com/userFRM/ThetaDataDx/sdks/go
```

Requires Go 1.21+ and a C compiler (for CGo).

## C++

First, build the Rust FFI library:

```bash
git clone https://github.com/userFRM/ThetaDataDx.git
cd ThetaDataDx
cargo build --release -p thetadatadx-ffi
```

Then build the C++ SDK with CMake:

```bash
cd sdks/cpp
mkdir build && cd build
cmake ..
make
```

Requires C++17, CMake 3.16+, and a C compiler.

## CLI Tool

Install the `tdx` command-line tool:

```bash
# From source
cargo install --path crates/thetadatadx-cli

# Or build from the workspace root
cargo build --release -p thetadatadx-cli
# binary at target/release/tdx
```

## MCP Server

Install the MCP server for LLM integration:

```bash
# From source
cargo install thetadatadx-mcp --git https://github.com/userFRM/ThetaDataDx

# Or build from the workspace
cd crates/thetadatadx-mcp
cargo build --release
# binary at ../../target/release/thetadatadx-mcp
```

## Prerequisites

| Language | Requirements |
|----------|-------------|
| Rust | Stable Rust toolchain |
| Python | Python 3.9+ |
| Go | Go 1.21+, C compiler, Rust toolchain (for FFI library) |
| C++ | C++17 compiler, CMake 3.16+, Rust toolchain (for FFI library) |
| CLI | Rust toolchain |
| MCP | Rust toolchain |

All languages require a valid [ThetaData](https://thetadata.us) subscription for accessing market data. The Greeks calculator works offline without any subscription.
