use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, ContentArrangement, Table};
use thetadatadx::types::price::Price;

// ═══════════════════════════════════════════════════════════════════════════
//  CLI argument types
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Parser)]
#[command(
    name = "tdx",
    version,
    about = "ThetaDataDx CLI — query ThetaData from your terminal",
    long_about = "Native CLI for ThetaData market data. No JVM required.\n\n\
                  Requires a creds.txt file (email on line 1, password on line 2)."
)]
struct Cli {
    /// Path to credentials file (email + password, one per line)
    #[arg(long, global = true, default_value = "creds.txt")]
    creds: String,

    /// Server configuration preset
    #[arg(long, global = true, default_value = "production")]
    config: ConfigPreset,

    /// Output format
    #[arg(long, global = true, default_value = "table")]
    format: OutputFormat,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum ConfigPreset {
    Production,
    Dev,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
    Csv,
}

#[derive(Subcommand)]
enum Commands {
    /// Test authentication and print session info
    Auth,
    /// Stock data commands
    #[command(subcommand)]
    Stock(StockCmd),
    /// Option data commands
    #[command(subcommand)]
    Option(OptionCmd),
    /// Index data commands
    #[command(subcommand)]
    Index(IndexCmd),
    /// Interest rate data commands
    #[command(subcommand)]
    Rate(RateCmd),
    /// Market calendar commands
    #[command(subcommand)]
    Calendar(CalendarCmd),
    /// Compute Black-Scholes Greeks (offline, no server needed)
    Greeks {
        /// Spot price
        spot: f64,
        /// Strike price
        strike: f64,
        /// Risk-free rate (e.g. 0.05)
        rate: f64,
        /// Dividend yield (e.g. 0.015)
        dividend: f64,
        /// Time to expiration in years (e.g. 0.082 for ~30 days)
        time: f64,
        /// Option price
        option_price: f64,
        /// Option type: call or put
        right: OptionRight,
    },
    /// Compute implied volatility only (offline, no server needed)
    Iv {
        /// Spot price
        spot: f64,
        /// Strike price
        strike: f64,
        /// Risk-free rate
        rate: f64,
        /// Dividend yield
        dividend: f64,
        /// Time to expiration in years
        time: f64,
        /// Option price
        option_price: f64,
        /// Option type: call or put
        right: OptionRight,
    },
}

#[derive(Clone, ValueEnum)]
enum OptionRight {
    Call,
    Put,
}

impl OptionRight {
    fn is_call(&self) -> bool {
        matches!(self, OptionRight::Call)
    }
}

// ── Stock subcommands ────────────────────────────────────────────────────

#[derive(Subcommand)]
enum StockCmd {
    /// List all available stock symbols
    ListSymbols,
    /// Fetch end-of-day data for a date range (YYYYMMDD)
    Eod {
        symbol: String,
        start: String,
        end: String,
    },
    /// Fetch intraday OHLC bars (interval in ms, e.g. 60000 for 1-min)
    Ohlc {
        symbol: String,
        date: String,
        interval: String,
    },
    /// Fetch all trades on a date
    Trade { symbol: String, date: String },
    /// Fetch NBBO quotes on a date (interval in ms)
    Quote {
        symbol: String,
        date: String,
        interval: String,
    },
    /// Live quote snapshot for one or more symbols (comma-separated)
    SnapshotQuote {
        /// Comma-separated symbols, e.g. AAPL,MSFT,GOOGL
        symbols: String,
    },
}

// ── Option subcommands ───────────────────────────────────────────────────

#[derive(Subcommand)]
enum OptionCmd {
    /// List available expiration dates for an underlying
    Expirations { symbol: String },
    /// List available strikes for an underlying at a given expiration
    Strikes {
        symbol: String,
        /// Expiration date (YYYYMMDD)
        expiration: String,
    },
    /// Fetch all trades for an option contract on a date
    Trade {
        symbol: String,
        /// Expiration (YYYYMMDD)
        expiration: String,
        /// Strike price as integer (e.g. 500000 for $500.00 at price_type 8)
        strike: String,
        /// C or P
        right: String,
        /// Trade date (YYYYMMDD)
        date: String,
    },
    /// Fetch NBBO quotes for an option contract on a date
    Quote {
        symbol: String,
        expiration: String,
        strike: String,
        right: String,
        date: String,
        interval: String,
    },
    /// Fetch end-of-day data for an option contract over a date range
    Eod {
        symbol: String,
        expiration: String,
        strike: String,
        right: String,
        start: String,
        end: String,
    },
}

// ── Index subcommands ────────────────────────────────────────────────────

#[derive(Subcommand)]
enum IndexCmd {
    /// List all available index symbols
    ListSymbols,
    /// Fetch end-of-day index data for a date range
    Eod {
        symbol: String,
        start: String,
        end: String,
    },
    /// Fetch intraday OHLC bars for an index
    Ohlc {
        symbol: String,
        start_date: String,
        end_date: String,
        interval: String,
    },
}

// ── Rate subcommands ─────────────────────────────────────────────────────

#[derive(Subcommand)]
enum RateCmd {
    /// Fetch end-of-day interest rate data for a date range
    Eod {
        symbol: String,
        start: String,
        end: String,
    },
}

// ── Calendar subcommands ─────────────────────────────────────────────────

#[derive(Subcommand)]
enum CalendarCmd {
    /// Check if the market is open today
    Today,
    /// Get full calendar for a year
    Year { year: String },
    /// Get calendar info for a specific date (YYYYMMDD)
    Date { date: String },
}

// ═══════════════════════════════════════════════════════════════════════════
//  Formatting helpers
// ═══════════════════════════════════════════════════════════════════════════

/// Format ms_of_day as HH:MM:SS.mmm
fn format_ms(ms: i32) -> String {
    if ms < 0 {
        return "N/A".to_string();
    }
    let total_ms = ms as u64;
    let h = total_ms / 3_600_000;
    let m = (total_ms % 3_600_000) / 60_000;
    let s = (total_ms % 60_000) / 1_000;
    let ms_frac = total_ms % 1_000;
    format!("{h:02}:{m:02}:{s:02}.{ms_frac:03}")
}

/// Format price from raw integer + price_type to a float string.
fn format_price(value: i32, price_type: i32) -> String {
    if price_type == 0 {
        return "0.00".to_string();
    }
    let p = Price::new(value, price_type);
    format!("{p}")
}

/// Format a YYYYMMDD integer date to a string.
fn format_date(date: i32) -> String {
    if date == 0 {
        return "N/A".to_string();
    }
    format!("{date}")
}

// ═══════════════════════════════════════════════════════════════════════════
//  Output renderers — one generic system for table / json / csv
// ═══════════════════════════════════════════════════════════════════════════

/// A row-oriented data structure that all output formatters consume.
struct TabularData {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl TabularData {
    fn new(headers: Vec<&str>) -> Self {
        Self {
            headers: headers.into_iter().map(|s| s.to_string()).collect(),
            rows: Vec::new(),
        }
    }

    fn push(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    fn render(&self, fmt: &OutputFormat) {
        match fmt {
            OutputFormat::Table => self.render_table(),
            OutputFormat::Json => self.render_json(),
            OutputFormat::Csv => self.render_csv(),
        }
    }

    fn render_table(&self) {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(self.headers.iter().map(Cell::new));

        for row in &self.rows {
            table.add_row(row.iter().map(Cell::new));
        }
        println!("{table}");
        eprintln!("{} rows", self.rows.len());
    }

    fn render_json(&self) {
        let arr: Vec<serde_json::Value> = self
            .rows
            .iter()
            .map(|row| {
                let mut map = serde_json::Map::new();
                for (i, h) in self.headers.iter().enumerate() {
                    let val = row.get(i).cloned().unwrap_or_default();
                    // Try to parse as number for cleaner JSON
                    if let Ok(n) = val.parse::<f64>() {
                        map.insert(
                            h.clone(),
                            serde_json::Value::Number(
                                serde_json::Number::from_f64(n)
                                    .unwrap_or_else(|| serde_json::Number::from(0)),
                            ),
                        );
                    } else {
                        map.insert(h.clone(), serde_json::Value::String(val));
                    }
                }
                serde_json::Value::Object(map)
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&arr).unwrap_or_else(|_| "[]".to_string())
        );
    }

    fn render_csv(&self) {
        println!("{}", self.headers.join(","));
        for row in &self.rows {
            println!("{}", row.join(","));
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════
//  DataTable renderer — for raw_endpoint results
// ═══════════════════════════════════════════════════════════════════════════

fn render_data_table(table: &thetadatadx::proto::DataTable, fmt: &OutputFormat) {
    let headers: Vec<&str> = table.headers.iter().map(|s| s.as_str()).collect();
    let mut td = TabularData::new(headers);

    for row in &table.data_table {
        let mut cells = Vec::new();
        for (i, header) in table.headers.iter().enumerate() {
            let cell = row
                .values
                .get(i)
                .and_then(|dv| dv.data_type.as_ref())
                .map(|dt| match dt {
                    thetadatadx::proto::data_value::DataType::Number(n) => format!("{n}"),
                    thetadatadx::proto::data_value::DataType::Text(t) => t.clone(),
                    thetadatadx::proto::data_value::DataType::Price(p) => {
                        format!("{}", Price::new(p.value, p.r#type))
                    }
                    thetadatadx::proto::data_value::DataType::NullValue(_) => String::new(),
                    _ => String::new(),
                })
                .unwrap_or_else(|| {
                    let _ = header;
                    String::new()
                });
            cells.push(cell);
        }
        td.push(cells);
    }

    td.render(fmt);
}

// ═══════════════════════════════════════════════════════════════════════════
//  Client construction helper
// ═══════════════════════════════════════════════════════════════════════════

async fn connect(
    creds_path: &str,
    preset: &ConfigPreset,
) -> Result<thetadatadx::DirectClient, thetadatadx::Error> {
    let creds = thetadatadx::Credentials::from_file(creds_path)?;
    let config = match preset {
        ConfigPreset::Production => thetadatadx::DirectConfig::production(),
        ConfigPreset::Dev => thetadatadx::DirectConfig::dev(),
    };
    thetadatadx::DirectClient::connect(&creds, config).await
}

// ═══════════════════════════════════════════════════════════════════════════
//  Main
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), thetadatadx::Error> {
    let fmt = &cli.format;

    match cli.command {
        // ── Auth ──────────────────────────────────────────────────────
        Commands::Auth => {
            let creds = thetadatadx::Credentials::from_file(&cli.creds)?;
            let resp = thetadatadx::auth::authenticate(&creds).await?;
            let mut td = TabularData::new(vec![
                "session_id",
                "email",
                "stock_tier",
                "options_tier",
                "indices_tier",
                "rate_tier",
                "created",
            ]);
            let user = resp.user.as_ref();
            td.push(vec![
                resp.session_id.clone(),
                user.and_then(|u| u.email.clone()).unwrap_or_default(),
                user.and_then(|u| u.stock_subscription)
                    .map(|t| format!("{t}"))
                    .unwrap_or_default(),
                user.and_then(|u| u.options_subscription)
                    .map(|t| format!("{t}"))
                    .unwrap_or_default(),
                user.and_then(|u| u.indices_subscription)
                    .map(|t| format!("{t}"))
                    .unwrap_or_default(),
                user.and_then(|u| u.interest_rate_subscription)
                    .map(|t| format!("{t}"))
                    .unwrap_or_default(),
                resp.session_created.unwrap_or_default(),
            ]);
            td.render(fmt);
        }

        // ── Greeks (offline) ─────────────────────────────────────────
        Commands::Greeks {
            spot,
            strike,
            rate,
            dividend,
            time,
            option_price,
            right,
        } => {
            let g = thetadatadx::greeks::all_greeks(
                spot,
                strike,
                rate,
                dividend,
                time,
                option_price,
                right.is_call(),
            );
            let mut td = TabularData::new(vec!["greek", "value"]);
            let rows = [
                ("value", g.value),
                ("iv", g.iv),
                ("iv_error", g.iv_error),
                ("delta", g.delta),
                ("gamma", g.gamma),
                ("theta", g.theta),
                ("vega", g.vega),
                ("rho", g.rho),
                ("d1", g.d1),
                ("d2", g.d2),
                ("vanna", g.vanna),
                ("charm", g.charm),
                ("vomma", g.vomma),
                ("veta", g.veta),
                ("speed", g.speed),
                ("zomma", g.zomma),
                ("color", g.color),
                ("ultima", g.ultima),
                ("dual_delta", g.dual_delta),
                ("dual_gamma", g.dual_gamma),
                ("epsilon", g.epsilon),
                ("lambda", g.lambda),
            ];
            for (name, val) in rows {
                td.push(vec![name.to_string(), format!("{val:.8}")]);
            }
            td.render(fmt);
        }

        // ── IV (offline) ─────────────────────────────────────────────
        Commands::Iv {
            spot,
            strike,
            rate,
            dividend,
            time,
            option_price,
            right,
        } => {
            let (iv, iv_error) = thetadatadx::greeks::implied_volatility(
                spot,
                strike,
                rate,
                dividend,
                time,
                option_price,
                right.is_call(),
            );
            let mut td = TabularData::new(vec!["iv", "iv_error"]);
            td.push(vec![format!("{iv:.8}"), format!("{iv_error:.8}")]);
            td.render(fmt);
        }

        // ── Stock ────────────────────────────────────────────────────
        Commands::Stock(sub) => {
            let client = connect(&cli.creds, &cli.config).await?;
            match sub {
                StockCmd::ListSymbols => {
                    let symbols = client.stock_list_symbols().await?;
                    let mut td = TabularData::new(vec!["symbol"]);
                    for s in &symbols {
                        td.push(vec![s.clone()]);
                    }
                    td.render(fmt);
                }
                StockCmd::Eod { symbol, start, end } => {
                    let ticks = client.stock_history_eod(&symbol, &start, &end).await?;
                    let mut td = TabularData::new(vec![
                        "date", "open", "high", "low", "close", "volume", "count", "bid", "ask",
                        "time",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_price(t.open, t.price_type),
                            format_price(t.high, t.price_type),
                            format_price(t.low, t.price_type),
                            format_price(t.close, t.price_type),
                            format!("{}", t.volume),
                            format!("{}", t.count),
                            format_price(t.bid, t.price_type),
                            format_price(t.ask, t.price_type),
                            format_ms(t.ms_of_day),
                        ]);
                    }
                    td.render(fmt);
                }
                StockCmd::Ohlc {
                    symbol,
                    date,
                    interval,
                } => {
                    let ticks = client.stock_history_ohlc(&symbol, &date, &interval).await?;
                    let mut td = TabularData::new(vec![
                        "date", "time", "open", "high", "low", "close", "volume", "count",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_ms(t.ms_of_day),
                            format_price(t.open, t.price_type),
                            format_price(t.high, t.price_type),
                            format_price(t.low, t.price_type),
                            format_price(t.close, t.price_type),
                            format!("{}", t.volume),
                            format!("{}", t.count),
                        ]);
                    }
                    td.render(fmt);
                }
                StockCmd::Trade { symbol, date } => {
                    let ticks = client.stock_history_trade(&symbol, &date).await?;
                    let mut td = TabularData::new(vec![
                        "date",
                        "time",
                        "price",
                        "size",
                        "exchange",
                        "condition",
                        "sequence",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_ms(t.ms_of_day),
                            format_price(t.price, t.price_type),
                            format!("{}", t.size),
                            format!("{}", t.exchange),
                            format!("{}", t.condition),
                            format!("{}", t.sequence),
                        ]);
                    }
                    td.render(fmt);
                }
                StockCmd::Quote {
                    symbol,
                    date,
                    interval,
                } => {
                    let ticks = client
                        .stock_history_quote(&symbol, &date, &interval)
                        .await?;
                    let mut td = TabularData::new(vec![
                        "date", "time", "bid", "bid_size", "ask", "ask_size",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_ms(t.ms_of_day),
                            format_price(t.bid, t.price_type),
                            format!("{}", t.bid_size),
                            format_price(t.ask, t.price_type),
                            format!("{}", t.ask_size),
                        ]);
                    }
                    td.render(fmt);
                }
                StockCmd::SnapshotQuote { symbols } => {
                    let syms: Vec<&str> = symbols.split(',').map(|s| s.trim()).collect();
                    let ticks = client.stock_snapshot_quote(&syms).await?;
                    let mut td = TabularData::new(vec![
                        "date", "time", "bid", "bid_size", "ask", "ask_size",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_ms(t.ms_of_day),
                            format_price(t.bid, t.price_type),
                            format!("{}", t.bid_size),
                            format_price(t.ask, t.price_type),
                            format!("{}", t.ask_size),
                        ]);
                    }
                    td.render(fmt);
                }
            }
        }

        // ── Option ───────────────────────────────────────────────────
        Commands::Option(sub) => {
            let client = connect(&cli.creds, &cli.config).await?;
            match sub {
                OptionCmd::Expirations { symbol } => {
                    let exps = client.option_list_expirations(&symbol).await?;
                    let mut td = TabularData::new(vec!["expiration"]);
                    for e in &exps {
                        td.push(vec![e.clone()]);
                    }
                    td.render(fmt);
                }
                OptionCmd::Strikes { symbol, expiration } => {
                    let strikes = client.option_list_strikes(&symbol, &expiration).await?;
                    let mut td = TabularData::new(vec!["strike"]);
                    for s in &strikes {
                        td.push(vec![s.clone()]);
                    }
                    td.render(fmt);
                }
                OptionCmd::Trade {
                    symbol,
                    expiration,
                    strike,
                    right,
                    date,
                } => {
                    let r = normalize_right(&right);
                    let ticks = client
                        .option_history_trade(&symbol, &expiration, &strike, r, &date)
                        .await?;
                    let mut td = TabularData::new(vec![
                        "date",
                        "time",
                        "price",
                        "size",
                        "exchange",
                        "condition",
                        "sequence",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_ms(t.ms_of_day),
                            format_price(t.price, t.price_type),
                            format!("{}", t.size),
                            format!("{}", t.exchange),
                            format!("{}", t.condition),
                            format!("{}", t.sequence),
                        ]);
                    }
                    td.render(fmt);
                }
                OptionCmd::Quote {
                    symbol,
                    expiration,
                    strike,
                    right,
                    date,
                    interval,
                } => {
                    let r = normalize_right(&right);
                    let ticks = client
                        .option_history_quote(&symbol, &expiration, &strike, r, &date, &interval)
                        .await?;
                    let mut td = TabularData::new(vec![
                        "date", "time", "bid", "bid_size", "ask", "ask_size",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_ms(t.ms_of_day),
                            format_price(t.bid, t.price_type),
                            format!("{}", t.bid_size),
                            format_price(t.ask, t.price_type),
                            format!("{}", t.ask_size),
                        ]);
                    }
                    td.render(fmt);
                }
                OptionCmd::Eod {
                    symbol,
                    expiration,
                    strike,
                    right,
                    start,
                    end,
                } => {
                    let r = normalize_right(&right);
                    let ticks = client
                        .option_history_eod(&symbol, &expiration, &strike, r, &start, &end)
                        .await?;
                    let mut td = TabularData::new(vec![
                        "date", "open", "high", "low", "close", "volume", "count", "bid", "ask",
                        "time",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_price(t.open, t.price_type),
                            format_price(t.high, t.price_type),
                            format_price(t.low, t.price_type),
                            format_price(t.close, t.price_type),
                            format!("{}", t.volume),
                            format!("{}", t.count),
                            format_price(t.bid, t.price_type),
                            format_price(t.ask, t.price_type),
                            format_ms(t.ms_of_day),
                        ]);
                    }
                    td.render(fmt);
                }
            }
        }

        // ── Index ────────────────────────────────────────────────────
        Commands::Index(sub) => {
            let client = connect(&cli.creds, &cli.config).await?;
            match sub {
                IndexCmd::ListSymbols => {
                    let symbols = client.index_list_symbols().await?;
                    let mut td = TabularData::new(vec!["symbol"]);
                    for s in &symbols {
                        td.push(vec![s.clone()]);
                    }
                    td.render(fmt);
                }
                IndexCmd::Eod { symbol, start, end } => {
                    let ticks = client.index_history_eod(&symbol, &start, &end).await?;
                    let mut td = TabularData::new(vec![
                        "date", "open", "high", "low", "close", "volume", "count", "bid", "ask",
                        "time",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_price(t.open, t.price_type),
                            format_price(t.high, t.price_type),
                            format_price(t.low, t.price_type),
                            format_price(t.close, t.price_type),
                            format!("{}", t.volume),
                            format!("{}", t.count),
                            format_price(t.bid, t.price_type),
                            format_price(t.ask, t.price_type),
                            format_ms(t.ms_of_day),
                        ]);
                    }
                    td.render(fmt);
                }
                IndexCmd::Ohlc {
                    symbol,
                    start_date,
                    end_date,
                    interval,
                } => {
                    let ticks = client
                        .index_history_ohlc(&symbol, &start_date, &end_date, &interval)
                        .await?;
                    let mut td = TabularData::new(vec![
                        "date", "time", "open", "high", "low", "close", "volume", "count",
                    ]);
                    for t in &ticks {
                        td.push(vec![
                            format_date(t.date),
                            format_ms(t.ms_of_day),
                            format_price(t.open, t.price_type),
                            format_price(t.high, t.price_type),
                            format_price(t.low, t.price_type),
                            format_price(t.close, t.price_type),
                            format!("{}", t.volume),
                            format!("{}", t.count),
                        ]);
                    }
                    td.render(fmt);
                }
            }
        }

        // ── Rate ─────────────────────────────────────────────────────
        Commands::Rate(sub) => {
            let client = connect(&cli.creds, &cli.config).await?;
            match sub {
                RateCmd::Eod { symbol, start, end } => {
                    let table = client
                        .interest_rate_history_eod(&symbol, &start, &end)
                        .await?;
                    render_data_table(&table, fmt);
                }
            }
        }

        // ── Calendar ─────────────────────────────────────────────────
        Commands::Calendar(sub) => {
            let client = connect(&cli.creds, &cli.config).await?;
            match sub {
                CalendarCmd::Today => {
                    let table = client.calendar_open_today().await?;
                    render_data_table(&table, fmt);
                }
                CalendarCmd::Year { year } => {
                    let table = client.calendar_year(&year).await?;
                    render_data_table(&table, fmt);
                }
                CalendarCmd::Date { date } => {
                    let table = client.calendar_on_date(&date).await?;
                    render_data_table(&table, fmt);
                }
            }
        }
    }

    Ok(())
}

/// Normalize option right to the uppercase single-letter format the API expects.
fn normalize_right(s: &str) -> &str {
    match s.to_uppercase().as_str() {
        "C" | "CALL" => "C",
        "P" | "PUT" => "P",
        // Return one of the static string slices; unknown values get "C" as a
        // fallback — the server will reject if truly invalid.
        _ => "C",
    }
}
