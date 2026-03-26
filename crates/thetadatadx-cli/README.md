# tdx - ThetaDataDx CLI

Command-line interface for querying ThetaData market data. No JVM required.

## Install

```bash
cargo install --path crates/thetadatadx-cli
```

Or build from the workspace root:

```bash
cargo build --release -p thetadatadx-cli
# binary at target/release/tdx
```

## Setup

Create a `creds.txt` file with your ThetaData credentials:

```
your-email@example.com
your-password
```

## Usage

```bash
# Test authentication
tdx auth --creds creds.txt

# Stock data
tdx stock list-symbols
tdx stock eod AAPL 20240101 20240301
tdx stock ohlc AAPL 20240315 60000           # 1-min bars
tdx stock trade AAPL 20240315
tdx stock quote AAPL 20240315 60000
tdx stock snapshot-quote AAPL,MSFT,GOOGL

# Options
tdx option expirations SPY
tdx option strikes SPY 20240419
tdx option trade SPY 20240419 500000 C 20240315
tdx option quote SPY 20240419 500000 C 20240315 60000
tdx option eod SPY 20240419 500000 C 20240101 20240301

# Indices
tdx index list-symbols
tdx index eod SPX 20240101 20240301
tdx index ohlc SPX 20240101 20240301 60000

# Interest rates
tdx rate eod SOFR 20240101 20240301

# Market calendar
tdx calendar today
tdx calendar year 2024
tdx calendar date 20240315

# Black-Scholes Greeks (offline, no server needed)
tdx greeks 450 450 0.05 0.015 0.082 8.5 call
tdx iv 450 450 0.05 0.015 0.082 8.5 call
```

## Output formats

```bash
tdx stock eod AAPL 20240101 20240301                  # pretty table (default)
tdx stock eod AAPL 20240101 20240301 --format json     # JSON array
tdx stock eod AAPL 20240101 20240301 --format csv      # CSV
```

## Global flags

| Flag | Default | Description |
|------|---------|-------------|
| `--creds <path>` | `creds.txt` | Credentials file |
| `--config <preset>` | `production` | `production` or `dev` |
| `--format <fmt>` | `table` | `table`, `json`, or `csv` |
