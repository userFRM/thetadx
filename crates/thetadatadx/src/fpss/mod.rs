//! FPSS (Feed Processing Streaming Server) real-time streaming client.
//!
//! # Architecture (from decompiled Java — `FPSSClient.java`)
//!
//! The FPSS protocol provides real-time market data over a custom TLS/TCP
//! binary protocol. The Java terminal's `FPSSClient` runs:
//!
//! 1. A TLS connection to one of 4 FPSS servers (NJ-A/NJ-B, ports 20000/20001)
//! 2. An authentication handshake (email + password over the wire)
//! 3. A heartbeat thread sending PING every 100ms
//! 4. A reader thread dispatching incoming frames to callbacks
//! 5. Automatic reconnection on disconnect (except for permanent errors)
//!
//! # Rust implementation — Disruptor architecture
//!
//! ```text
//!  ┌─────────────┐        ┌──────────────┐        ┌──────────────────┐
//!  │ FpssClient  │───────►│ Async read   │──SyncCh─►│ Disruptor ring │
//!  │             │        │ loop (tokio) │        │ (pre-allocated,  │
//!  │ .subscribe()│        └──────────────┘        │  lock-free SPSC) │
//!  │ .unsubscribe│        ┌──────────────┐        └────────┬─────────┘
//!  │ .shutdown() │───────►│ Ping task    │                 │ consumer
//!  └─────────────┘        └──────────────┘                 ▼
//!                                                 ┌──────────────────┐
//!                                                 │ tokio::mpsc      │
//!                                                 │ (async consumer) │
//!                                                 └──────────────────┘
//! ```
//!
//! The event dispatch pipeline uses an LMAX Disruptor ring buffer (via
//! `disruptor-rs`) for lock-free, pre-allocated event processing. This mirrors
//! the Java terminal's own Disruptor architecture:
//! - Java: blocking `DataInputStream` → LMAX Disruptor ring → event handlers
//! - Rust: async TLS read → bounded sync channel → Disruptor ring → tokio mpsc
//!
//! The Disruptor eliminates per-event atomic contention that `tokio::sync::mpsc`
//! incurs on the hot path. Events are pre-allocated in the ring buffer (zero
//! allocation for slot metadata), and the single-producer barrier uses a plain
//! store (no CAS) for publication.
//!
//! Callers still receive events through `tokio::sync::mpsc::Receiver<FpssEvent>`,
//! preserving full async compatibility.
//!
//! # Sub-modules
//!
//! - [`connection`] — TLS TCP connection establishment
//! - [`framing`] — Wire frame reader/writer (1-byte len + 1-byte code + payload)
//! - [`protocol`] — Message types, contract serialization, subscription payloads
//! - [`ring`] — LMAX Disruptor ring buffer and adaptive wait strategy

pub mod connection;
pub mod framing;
pub mod protocol;
pub mod ring;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex, Notify};
use tokio::task::JoinHandle;

use self::ring::{RingPipeline, RingPublisher};

use crate::auth::Credentials;
use crate::error::Error;
use crate::types::enums::{RemoveReason, StreamMsgType, StreamResponseType};

use self::connection::FpssWriter;
use self::framing::{write_frame, write_raw_frame, Frame};
use self::protocol::{
    build_credentials_payload, build_ping_payload, build_subscribe_payload, parse_contract_message,
    parse_disconnect_reason, parse_req_response, Contract, SubscriptionKind, PING_INTERVAL_MS,
    RECONNECT_DELAY_MS, TOO_MANY_REQUESTS_DELAY_MS,
};

/// Events emitted by the FPSS background read loop.
///
/// Subscribers receive these through their channels. The enum is non-exhaustive
/// to allow adding new event types without breaking downstream.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum FpssEvent {
    /// Login succeeded. Payload is the permissions string from METADATA (code 3).
    ///
    /// Source: `FPSSClient.onMetadata()` — server sends permissions as UTF-8.
    LoginSuccess { permissions: String },

    /// Server sent a CONTRACT assignment (code 20).
    ///
    /// The server assigns a numeric ID to each contract on first subscription.
    /// Subsequent data messages reference this ID instead of the full contract.
    ///
    /// Source: `FPSSClient.onContract()`.
    ContractAssigned { id: i32, contract: Contract },

    /// Raw quote data arrived (code 21, server->client direction).
    ///
    /// Payload is the raw FIT-encoded bytes. The caller must decode using
    /// `codec::fit::FitReader` once that module is available.
    ///
    /// Source: `FPSSClient.onQuote()` — dispatches to registered listeners.
    QuoteData { payload: Vec<u8> },

    /// Raw trade data arrived (code 22, server->client direction).
    ///
    /// Source: `FPSSClient.onTrade()`.
    TradeData { payload: Vec<u8> },

    /// Raw open interest data arrived (code 23, server->client direction).
    ///
    /// Source: `FPSSClient.onOpenInterest()`.
    OpenInterestData { payload: Vec<u8> },

    /// Raw OHLCVC snapshot arrived (code 24, server->client direction).
    ///
    /// Source: `FPSSClient.onOHLCVC()`.
    OhlcvcData { payload: Vec<u8> },

    /// Subscription response (code 40).
    ///
    /// Source: `FPSSClient.onReqResponse()`.
    ReqResponse {
        req_id: i32,
        result: StreamResponseType,
    },

    /// Market open signal (code 30).
    ///
    /// Source: `FPSSClient.onStart()`.
    MarketOpen,

    /// Market close / stop signal (code 32).
    ///
    /// Source: `FPSSClient.onStop()`.
    MarketClose,

    /// Server error message (code 11). Payload is UTF-8 error text.
    ///
    /// Source: `FPSSClient.onError()`.
    ServerError { message: String },

    /// Server disconnected us (code 12). Contains the parsed reason.
    ///
    /// Source: `FPSSClient.onDisconnected()`.
    Disconnected { reason: RemoveReason },

    /// Protocol-level parse error (e.g. malformed CONTRACT or REQ_RESPONSE).
    ///
    /// Callers should log these; they indicate protocol-level corruption or
    /// version mismatch with the server.
    Error { message: String },
}

/// Shared state between the client handle and background tasks.
struct SharedState {
    /// Writer half of the TLS connection, protected by a mutex for
    /// concurrent access from the ping task and client methods.
    writer: Mutex<FpssWriter>,

    /// Monotonically increasing request ID counter.
    /// Source: `PacketStream.java` uses an AtomicInteger for req_id generation.
    next_req_id: AtomicI32,

    /// Whether we are authenticated and the connection is live.
    authenticated: AtomicBool,

    /// Signal to shut down background tasks.
    shutdown: Notify,

    /// Whether shutdown has been requested.
    is_shutdown: AtomicBool,

    /// Active subscriptions for reconnection.
    /// Maps (kind, contract) -> req_id for re-subscribing after reconnect.
    ///
    /// Source: `FPSSClient.java` — maintains subscription list for reconnection,
    /// re-sends all with `req_id = -1` on reconnect.
    active_subs: Mutex<Vec<(SubscriptionKind, Contract)>>,

    /// Server-assigned contract ID mapping.
    /// Maps contract_id -> Contract for decoding data messages.
    ///
    /// Source: `FPSSClient.java` — `contractMap` field.
    contract_map: Mutex<HashMap<i32, Contract>>,
}

/// Real-time streaming client for ThetaData's FPSS servers.
///
/// # Lifecycle (from `FPSSClient.java`)
///
/// 1. `FpssClient::connect()` — TLS connect + authenticate + start background tasks
/// 2. `subscribe_quotes()` / `subscribe_trades()` — subscribe to market data
/// 3. Receive events through the returned channel or `event_receiver()`
/// 4. `shutdown()` — clean disconnect
///
/// # Reconnection
///
/// Reconnection is **manual** — the caller must monitor `FpssEvent::Disconnected`
/// events and invoke [`reconnect()`] explicitly. The read loop emits a disconnect
/// event on I/O errors and read timeouts, but does not automatically reconnect.
///
/// On reconnect, all active subscriptions are re-sent with `req_id = -1`
/// (matching Java behavior). Permanent errors (`AccountAlreadyConnected`) should
/// NOT be retried.
///
/// Source: `FPSSClient.java` — main connection/reconnection state machine.
pub struct FpssClient {
    state: Arc<SharedState>,
    /// Tokio mpsc sender for creating additional subscribers. The disruptor
    /// consumer forwards events to this channel; async callers receive from
    /// the paired `mpsc::Receiver`.
    event_tx: mpsc::Sender<FpssEvent>,
    /// Handle to the background reader task.
    reader_handle: Option<JoinHandle<()>>,
    /// Handle to the ping heartbeat task.
    ping_handle: Option<JoinHandle<()>>,
    /// LMAX Disruptor ring buffer pipeline (feeder + consumer threads).
    /// Dropped on shutdown to join background threads.
    _ring_pipeline: RingPipeline,
    /// The server address we connected to.
    server_addr: String,
}

impl FpssClient {
    /// Connect to a ThetaData FPSS server, authenticate, and start background tasks.
    ///
    /// # Sequence (from `FPSSClient.java`)
    ///
    /// 1. Try each server in `SERVERS` until one connects (TLS over TCP)
    /// 2. Send CREDENTIALS (code 0) with email + password
    /// 3. Wait for METADATA (code 3) = login success, or DISCONNECTED (code 12) = failure
    /// 4. Start ping heartbeat (100ms interval)
    /// 5. Start background reader loop
    ///
    /// Returns the client and an event receiver channel.
    ///
    /// Source: `FPSSClient.connect()` and `FPSSClient.sendCredentials()`.
    pub async fn connect(
        creds: &Credentials,
        event_buffer: usize,
    ) -> Result<(Self, mpsc::Receiver<FpssEvent>), Error> {
        let (reader, writer, server_addr) = connection::connect().await?;
        Self::connect_with_transport(creds, reader, writer, server_addr, event_buffer).await
    }

    /// Connect using pre-established transport (useful for testing with mock sockets).
    pub(crate) async fn connect_with_transport(
        creds: &Credentials,
        mut reader: connection::FpssReader,
        writer: connection::FpssWriter,
        server_addr: String,
        event_buffer: usize,
    ) -> Result<(Self, mpsc::Receiver<FpssEvent>), Error> {
        let (event_tx, event_rx) = mpsc::channel(event_buffer);

        // Build the LMAX Disruptor ring buffer pipeline.
        // The ring_size is derived from event_buffer, rounded up to a power of 2.
        let (ring_publisher, ring_pipeline) =
            ring::build_ring_pipeline(event_tx.clone(), event_buffer);

        let state = Arc::new(SharedState {
            writer: Mutex::new(writer),
            next_req_id: AtomicI32::new(1),
            authenticated: AtomicBool::new(false),
            shutdown: Notify::new(),
            is_shutdown: AtomicBool::new(false),
            active_subs: Mutex::new(Vec::new()),
            contract_map: Mutex::new(HashMap::new()),
        });

        // Send CREDENTIALS (code 0)
        // Source: FPSSClient.sendCredentials()
        let cred_payload = build_credentials_payload(&creds.email, &creds.password);
        {
            let mut w = state.writer.lock().await;
            let frame = Frame::new(StreamMsgType::Credentials, cred_payload);
            write_frame(&mut *w, &frame).await?;
        }
        tracing::debug!("sent CREDENTIALS to {server_addr}");

        // Wait for METADATA (success) or DISCONNECTED (failure)
        // Source: FPSSClient.connect() — blocks until login response arrives
        let login_result = Self::wait_for_login(&mut reader).await?;

        match login_result {
            LoginResult::Success(permissions) => {
                tracing::info!(
                    server = %server_addr,
                    permissions = %permissions,
                    "FPSS login successful"
                );
                state.authenticated.store(true, Ordering::Release);

                // Send login success event through the disruptor ring.
                let _ = ring_publisher.send(FpssEvent::LoginSuccess {
                    permissions: permissions.clone(),
                });
            }
            LoginResult::Disconnected(reason) => {
                return Err(Error::FpssDisconnected(format!(
                    "server rejected login: {reason:?}"
                )));
            }
        }

        // Start background tasks.
        // The read loop now publishes into the disruptor ring (via RingPublisher)
        // instead of directly into the tokio mpsc channel.
        let reader_state = Arc::clone(&state);
        let reader_handle = tokio::spawn(async move {
            read_loop(reader, reader_state, ring_publisher).await;
        });

        let ping_state = Arc::clone(&state);
        let ping_handle = tokio::spawn(async move {
            ping_loop(ping_state).await;
        });

        let client = FpssClient {
            state,
            event_tx,
            reader_handle: Some(reader_handle),
            ping_handle: Some(ping_handle),
            _ring_pipeline: ring_pipeline,
            server_addr,
        };

        Ok((client, event_rx))
    }

    /// Wait for the server's login response.
    ///
    /// Source: `FPSSClient.connect()` — reads frames until METADATA or DISCONNECTED.
    async fn wait_for_login(reader: &mut connection::FpssReader) -> Result<LoginResult, Error> {
        // Read frames until we get METADATA or DISCONNECTED
        // The server may send other frames first (unlikely during login, but handle it)
        loop {
            let frame = framing::read_frame(reader).await?.ok_or_else(|| {
                Error::Fpss("connection closed during login handshake".to_string())
            })?;

            match frame.code {
                StreamMsgType::Metadata => {
                    // Login success — payload is UTF-8 permissions string
                    let permissions = String::from_utf8_lossy(&frame.payload).to_string();
                    return Ok(LoginResult::Success(permissions));
                }
                StreamMsgType::Disconnected => {
                    let reason = parse_disconnect_reason(&frame.payload);
                    return Ok(LoginResult::Disconnected(reason));
                }
                StreamMsgType::Error => {
                    let msg = String::from_utf8_lossy(&frame.payload);
                    tracing::warn!(message = %msg, "server error during login");
                    return Err(Error::Fpss(format!("server error during login: {msg}")));
                }
                other => {
                    tracing::trace!(code = ?other, "ignoring frame during login handshake");
                }
            }
        }
    }

    /// Subscribe to quote data for a contract.
    ///
    /// Returns the request ID assigned to this subscription. The caller receives
    /// data through the event channel returned by `connect()`.
    ///
    /// # Wire protocol (from `PacketStream.addQuote()`)
    ///
    /// Sends code 21 (QUOTE) with payload `[req_id: i32 BE] [contract bytes]`.
    /// Server responds with code 40 (REQ_RESPONSE).
    pub async fn subscribe_quotes(&self, contract: &Contract) -> Result<i32, Error> {
        self.subscribe(SubscriptionKind::Quote, contract).await
    }

    /// Subscribe to trade data for a contract.
    ///
    /// Source: `PacketStream.addTrade()` — sends code 22 (TRADE).
    pub async fn subscribe_trades(&self, contract: &Contract) -> Result<i32, Error> {
        self.subscribe(SubscriptionKind::Trade, contract).await
    }

    /// Subscribe to open interest data for a contract.
    ///
    /// Source: `PacketStream.addOpenInterest()` — sends code 23 (OPEN_INTEREST).
    pub async fn subscribe_open_interest(&self, contract: &Contract) -> Result<i32, Error> {
        self.subscribe(SubscriptionKind::OpenInterest, contract)
            .await
    }

    /// Subscribe to all trades for a security type (full trade stream).
    ///
    /// # Wire protocol (from `PacketStream.java`)
    ///
    /// Sends code 22 (TRADE) with 5-byte payload `[req_id: i32 BE] [sec_type: u8]`.
    /// The server distinguishes this from per-contract subscriptions by payload length.
    pub async fn subscribe_full_trades(
        &self,
        sec_type: crate::types::enums::SecType,
    ) -> Result<i32, Error> {
        self.check_connected()?;

        let req_id = self.state.next_req_id.fetch_add(1, Ordering::Relaxed);
        let payload = protocol::build_full_type_subscribe_payload(req_id, sec_type);

        let frame = Frame::new(StreamMsgType::Trade, payload);
        let mut w = self.state.writer.lock().await;
        write_frame(&mut *w, &frame).await?;

        tracing::debug!(req_id, sec_type = ?sec_type, "sent full trade subscription");
        Ok(req_id)
    }

    /// Unsubscribe from quote data for a contract.
    ///
    /// Source: `PacketStream.removeQuote()` — sends code 51 (REMOVE_QUOTE).
    pub async fn unsubscribe_quotes(&self, contract: &Contract) -> Result<i32, Error> {
        self.unsubscribe(SubscriptionKind::Quote, contract).await
    }

    /// Unsubscribe from trade data for a contract.
    ///
    /// Source: `PacketStream.removeTrade()` — sends code 52 (REMOVE_TRADE).
    pub async fn unsubscribe_trades(&self, contract: &Contract) -> Result<i32, Error> {
        self.unsubscribe(SubscriptionKind::Trade, contract).await
    }

    /// Unsubscribe from open interest data for a contract.
    ///
    /// Source: `PacketStream.removeOpenInterest()` — sends code 53.
    pub async fn unsubscribe_open_interest(&self, contract: &Contract) -> Result<i32, Error> {
        self.unsubscribe(SubscriptionKind::OpenInterest, contract)
            .await
    }

    /// Internal subscribe implementation.
    async fn subscribe(&self, kind: SubscriptionKind, contract: &Contract) -> Result<i32, Error> {
        self.check_connected()?;

        let req_id = self.state.next_req_id.fetch_add(1, Ordering::Relaxed);
        let payload = build_subscribe_payload(req_id, contract);
        let code = kind.subscribe_code();

        let frame = Frame::new(code, payload);
        {
            let mut w = self.state.writer.lock().await;
            write_frame(&mut *w, &frame).await?;
        }

        // Track for reconnection
        {
            let mut subs = self.state.active_subs.lock().await;
            subs.push((kind, contract.clone()));
        }

        tracing::debug!(
            req_id,
            kind = ?kind,
            contract = %contract,
            "sent subscription"
        );
        Ok(req_id)
    }

    /// Internal unsubscribe implementation.
    async fn unsubscribe(&self, kind: SubscriptionKind, contract: &Contract) -> Result<i32, Error> {
        self.check_connected()?;

        let req_id = self.state.next_req_id.fetch_add(1, Ordering::Relaxed);
        let payload = build_subscribe_payload(req_id, contract);
        let code = kind.unsubscribe_code();

        let frame = Frame::new(code, payload);
        {
            let mut w = self.state.writer.lock().await;
            write_frame(&mut *w, &frame).await?;
        }

        // Remove from tracked subscriptions
        {
            let mut subs = self.state.active_subs.lock().await;
            subs.retain(|(k, c)| !(k == &kind && c == contract));
        }

        tracing::debug!(
            req_id,
            kind = ?kind,
            contract = %contract,
            "sent unsubscribe"
        );
        Ok(req_id)
    }

    /// Send the STOP message and shut down background tasks.
    ///
    /// Source: `FPSSClient.disconnect()` — sends STOP (code 32), then closes socket.
    pub async fn shutdown(&mut self) -> Result<(), Error> {
        if self.state.is_shutdown.swap(true, Ordering::AcqRel) {
            return Ok(()); // already shut down
        }

        tracing::info!(server = %self.server_addr, "shutting down FPSS client");

        // Send STOP to server
        let stop_payload = protocol::build_stop_payload();
        {
            let mut w = self.state.writer.lock().await;
            let _ = write_raw_frame(&mut *w, StreamMsgType::Stop, &stop_payload).await;
        }

        // Signal background tasks to stop
        self.state.shutdown.notify_waiters();

        // Wait for tasks to finish
        if let Some(h) = self.reader_handle.take() {
            h.abort();
            let _ = h.await;
        }
        if let Some(h) = self.ping_handle.take() {
            h.abort();
            let _ = h.await;
        }

        self.state.authenticated.store(false, Ordering::Release);
        tracing::info!("FPSS client shut down");
        Ok(())
    }

    /// Check if the client is currently authenticated.
    pub fn is_authenticated(&self) -> bool {
        self.state.authenticated.load(Ordering::Acquire)
    }

    /// Get the server address we are connected to.
    pub fn server_addr(&self) -> &str {
        &self.server_addr
    }

    /// Get the current contract map (server-assigned IDs -> contracts).
    ///
    /// Useful for decoding data messages that reference contracts by ID.
    pub async fn contract_map(&self) -> HashMap<i32, Contract> {
        self.state.contract_map.lock().await.clone()
    }

    /// Look up a single contract by its server-assigned ID.
    ///
    /// Much cheaper than [`contract_map()`](Self::contract_map) for the hot path
    /// where callers decode FIT ticks and need to resolve individual contract IDs.
    pub async fn contract_lookup(&self, id: i32) -> Option<Contract> {
        self.state.contract_map.lock().await.get(&id).cloned()
    }

    /// Get a clone of the event sender (for creating additional subscribers).
    pub fn event_sender(&self) -> mpsc::Sender<FpssEvent> {
        self.event_tx.clone()
    }

    /// Verify connection is live before sending.
    fn check_connected(&self) -> Result<(), Error> {
        if self.state.is_shutdown.load(Ordering::Acquire) {
            return Err(Error::Fpss("client is shut down".to_string()));
        }
        if !self.state.authenticated.load(Ordering::Acquire) {
            return Err(Error::Fpss("not authenticated".to_string()));
        }
        Ok(())
    }
}

impl Drop for FpssClient {
    fn drop(&mut self) {
        // Abort background tasks if not already shut down.
        // We can't await here, so just abort.
        if let Some(h) = self.reader_handle.take() {
            h.abort();
        }
        if let Some(h) = self.ping_handle.take() {
            h.abort();
        }
    }
}

// ---------------------------------------------------------------------------
// Login result (internal)
// ---------------------------------------------------------------------------

enum LoginResult {
    Success(String),
    Disconnected(RemoveReason),
}

// ---------------------------------------------------------------------------
// Background read loop
// ---------------------------------------------------------------------------

/// Background task that reads FPSS frames and dispatches events.
///
/// # Behavior (from `FPSSClient.java` reader thread)
///
/// Reads frames in a loop. On each frame:
/// - METADATA (3): login success (already handled before this loop starts)
/// - CONTRACT (20): parse and store contract_id -> contract mapping
/// - QUOTE (21): forward raw FIT payload to subscribers
/// - TRADE (22): forward raw FIT payload
/// - OPEN_INTEREST (23): forward raw payload
/// - OHLCVC (24): forward raw FIT payload
/// - REQ_RESPONSE (40): parse req_id + response code, notify subscribers
/// - START (30): market open signal
/// - STOP (32): market close signal
/// - ERROR (11): log and forward error text
/// - DISCONNECTED (12): parse reason, decide reconnect vs permanent stop
/// - PING (10): ignored (server doesn't send pings to client)
///
/// On I/O error: emits Disconnected event with Unspecified reason.
///
/// Events are published into the LMAX Disruptor ring buffer via [`RingPublisher`],
/// which forwards them through the lock-free ring to the async consumer channel.
///
/// Source: `FPSSClient.java` reader thread + all `on*()` callback methods.
async fn read_loop(
    mut reader: connection::FpssReader,
    state: Arc<SharedState>,
    ring_tx: RingPublisher,
) {
    // Read timeout matching Java's SO_TIMEOUT=10s (configurable via
    // protocol::READ_TIMEOUT_MS). On timeout, treat as disconnect.
    let read_timeout = Duration::from_millis(protocol::READ_TIMEOUT_MS);

    loop {
        if state.is_shutdown.load(Ordering::Acquire) {
            break;
        }

        let frame_result =
            tokio::time::timeout(read_timeout, framing::read_frame(&mut reader)).await;

        let frame = match frame_result {
            Ok(Ok(Some(frame))) => frame,
            Ok(Ok(None)) => {
                // Clean EOF
                tracing::warn!("FPSS connection closed by server");
                let _ = ring_tx.send(FpssEvent::Disconnected {
                    reason: RemoveReason::Unspecified,
                });
                state.authenticated.store(false, Ordering::Release);
                break;
            }
            Ok(Err(e)) => {
                tracing::error!(error = %e, "FPSS read error");
                let _ = ring_tx.send(FpssEvent::Disconnected {
                    reason: RemoveReason::Unspecified,
                });
                state.authenticated.store(false, Ordering::Release);
                break;
            }
            Err(_) => {
                // Read timeout — treat as disconnect, trigger reconnect
                tracing::warn!(
                    timeout_ms = protocol::READ_TIMEOUT_MS,
                    "FPSS read timed out"
                );
                let _ = ring_tx.send(FpssEvent::Disconnected {
                    reason: RemoveReason::TimedOut,
                });
                state.authenticated.store(false, Ordering::Release);
                break;
            }
        };

        let event = match frame.code {
            StreamMsgType::Metadata => {
                // Can arrive again after reconnection
                let permissions = String::from_utf8_lossy(&frame.payload).to_string();
                tracing::debug!(permissions = %permissions, "received METADATA");
                state.authenticated.store(true, Ordering::Release);
                Some(FpssEvent::LoginSuccess { permissions })
            }

            StreamMsgType::Contract => match parse_contract_message(&frame.payload) {
                Ok((id, contract)) => {
                    tracing::debug!(id, contract = %contract, "contract assigned");
                    state.contract_map.lock().await.insert(id, contract.clone());
                    Some(FpssEvent::ContractAssigned { id, contract })
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to parse CONTRACT message");
                    Some(FpssEvent::Error {
                        message: format!("failed to parse CONTRACT message: {e}"),
                    })
                }
            },

            StreamMsgType::Quote => Some(FpssEvent::QuoteData {
                payload: frame.payload,
            }),

            StreamMsgType::Trade => Some(FpssEvent::TradeData {
                payload: frame.payload,
            }),

            StreamMsgType::OpenInterest => Some(FpssEvent::OpenInterestData {
                payload: frame.payload,
            }),

            StreamMsgType::Ohlcvc => Some(FpssEvent::OhlcvcData {
                payload: frame.payload,
            }),

            StreamMsgType::ReqResponse => match parse_req_response(&frame.payload) {
                Ok((req_id, result)) => {
                    tracing::debug!(req_id, result = ?result, "subscription response");
                    Some(FpssEvent::ReqResponse { req_id, result })
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to parse REQ_RESPONSE");
                    Some(FpssEvent::Error {
                        message: format!("failed to parse REQ_RESPONSE: {e}"),
                    })
                }
            },

            StreamMsgType::Start => {
                tracing::info!("market open signal received");
                Some(FpssEvent::MarketOpen)
            }

            StreamMsgType::Stop => {
                tracing::info!("market close signal received");
                Some(FpssEvent::MarketClose)
            }

            StreamMsgType::Error => {
                let message = String::from_utf8_lossy(&frame.payload).to_string();
                tracing::warn!(message = %message, "server error");
                Some(FpssEvent::ServerError { message })
            }

            StreamMsgType::Disconnected => {
                let reason = parse_disconnect_reason(&frame.payload);
                tracing::warn!(reason = ?reason, "server disconnected us");
                state.authenticated.store(false, Ordering::Release);

                // Permanent errors — no reconnect will fix these.
                if reconnect_delay(reason).is_none() {
                    tracing::error!(reason = ?reason, "permanent disconnect — stopping");
                    state.is_shutdown.store(true, Ordering::Release);
                }

                Some(FpssEvent::Disconnected { reason })
            }

            // Ignore frame types we don't handle (e.g., server sending PING)
            other => {
                tracing::trace!(code = ?other, "ignoring unhandled frame type");
                None
            }
        };

        if let Some(evt) = event {
            // Publish into the disruptor ring. If the ring is full or the
            // pipeline is shut down, the event is silently discarded (matching
            // the previous `let _ = event_tx.send(evt).await` behavior).
            let _ = ring_tx.send(evt);
        }
    }
}

// ---------------------------------------------------------------------------
// Ping heartbeat loop
// ---------------------------------------------------------------------------

/// Background task that sends PING heartbeat every 100ms.
///
/// # Behavior (from `FPSSClient.java`)
///
/// After successful login, the Java client starts a thread that sends:
/// - Code 10 (PING)
/// - 1-byte payload: `[0x00]`
/// - Every 100ms
///
/// If the write fails, the task stops (the read loop will detect the broken connection).
///
/// Source: `FPSSClient.java` heartbeat thread, interval = 100ms.
async fn ping_loop(state: Arc<SharedState>) {
    let ping_payload = build_ping_payload();
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(PING_INTERVAL_MS));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                if state.is_shutdown.load(Ordering::Acquire) {
                    break;
                }
                if !state.authenticated.load(Ordering::Acquire) {
                    // Don't send pings if not authenticated
                    continue;
                }

                let mut w = state.writer.lock().await;
                if let Err(e) = write_raw_frame(&mut *w, StreamMsgType::Ping, &ping_payload).await {
                    tracing::warn!(error = %e, "ping write failed, stopping heartbeat");
                    break;
                }
            }
            _ = state.shutdown.notified() => {
                tracing::debug!("ping loop received shutdown signal");
                break;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Reconnection helper
// ---------------------------------------------------------------------------

/// Reconnect an FPSS client after a disconnect.
///
/// # Behavior (from `FPSSClient.java`)
///
/// 1. Wait `delay_ms` before attempting reconnection
/// 2. Establish a new TLS connection
/// 3. Re-authenticate
/// 4. Re-subscribe all previously active subscriptions with `req_id = -1`
///
/// On `TOO_MANY_REQUESTS`: wait 130 seconds before reconnecting.
/// On `ACCOUNT_ALREADY_CONNECTED`: do NOT reconnect (permanent error).
///
/// Source: `FPSSClient.java` reconnection logic in the main loop.
pub async fn reconnect(
    creds: &Credentials,
    previous_subs: Vec<(SubscriptionKind, Contract)>,
    delay_ms: u64,
    event_buffer: usize,
) -> Result<(FpssClient, mpsc::Receiver<FpssEvent>), Error> {
    tracing::info!(delay_ms, "waiting before FPSS reconnection");
    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;

    let (client, rx) = FpssClient::connect(creds, event_buffer).await?;

    // Re-subscribe all previous subscriptions with req_id = -1
    // Source: FPSSClient.java — reconnect logic uses req_id = -1 for re-subscriptions
    for (kind, contract) in &previous_subs {
        let payload = build_subscribe_payload(-1, contract);
        let code = kind.subscribe_code();
        let frame = Frame::new(code, payload);
        {
            let mut w = client.state.writer.lock().await;
            write_frame(&mut *w, &frame).await?;
        }
        tracing::debug!(
            kind = ?kind,
            contract = %contract,
            "re-subscribed after reconnect (req_id=-1)"
        );
    }

    // Store the re-subscribed list
    {
        let mut subs = client.state.active_subs.lock().await;
        *subs = previous_subs;
    }

    Ok((client, rx))
}

/// Determine the reconnect delay based on the disconnect reason.
///
/// Source: `FPSSClient.java` — reconnect logic checks `RemoveReason` to decide delay.
pub fn reconnect_delay(reason: RemoveReason) -> Option<u64> {
    match reason {
        // Permanent errors — no amount of reconnection will fix bad credentials.
        RemoveReason::AccountAlreadyConnected
        | RemoveReason::InvalidCredentials
        | RemoveReason::InvalidLoginValues
        | RemoveReason::InvalidLoginSize
        | RemoveReason::FreeAccount
        | RemoveReason::ServerUserDoesNotExist
        | RemoveReason::InvalidCredentialsNullUser => None,
        RemoveReason::TooManyRequests => Some(TOO_MANY_REQUESTS_DELAY_MS),
        _ => Some(RECONNECT_DELAY_MS),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconnect_delay_permanent() {
        // All credential / account errors are permanent — no reconnect.
        assert_eq!(reconnect_delay(RemoveReason::AccountAlreadyConnected), None);
        assert_eq!(reconnect_delay(RemoveReason::InvalidCredentials), None);
        assert_eq!(reconnect_delay(RemoveReason::InvalidLoginValues), None);
        assert_eq!(reconnect_delay(RemoveReason::InvalidLoginSize), None);
        assert_eq!(reconnect_delay(RemoveReason::FreeAccount), None);
        assert_eq!(reconnect_delay(RemoveReason::ServerUserDoesNotExist), None);
        assert_eq!(
            reconnect_delay(RemoveReason::InvalidCredentialsNullUser),
            None
        );
    }

    #[test]
    fn reconnect_delay_too_many_requests() {
        assert_eq!(
            reconnect_delay(RemoveReason::TooManyRequests),
            Some(130_000)
        );
    }

    #[test]
    fn reconnect_delay_normal() {
        assert_eq!(reconnect_delay(RemoveReason::ServerRestarting), Some(2_000));
        assert_eq!(reconnect_delay(RemoveReason::Unspecified), Some(2_000));
        assert_eq!(reconnect_delay(RemoveReason::TimedOut), Some(2_000));
    }
}
