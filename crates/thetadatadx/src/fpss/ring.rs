//! LMAX Disruptor ring buffer for lock-free FPSS event dispatch.
//!
//! # Architecture
//!
//! ```text
//!  ┌──────────────────┐     std::sync::mpsc      ┌──────────────────┐
//!  │  Async read loop │ ─────────────────────────►│ Disruptor feeder │
//!  │  (tokio task)    │     (bounded, lock-free)  │ (std::thread)    │
//!  └──────────────────┘                           └────────┬─────────┘
//!                                                          │ publish()
//!                                                          ▼
//!                                                 ┌──────────────────┐
//!                                                 │ Disruptor Ring   │
//!                                                 │ (pre-allocated,  │
//!                                                 │  lock-free SPSC) │
//!                                                 └────────┬─────────┘
//!                                                          │ consumer
//!                                                          ▼
//!                                                 ┌──────────────────┐
//!                                                 │ Event handler    │
//!                                                 │ (std::thread)    │──► tokio::sync::mpsc
//!                                                 └──────────────────┘     to async consumer
//! ```
//!
//! This mirrors the Java terminal's LMAX Disruptor architecture:
//! - Java: blocking `DataInputStream` -> Disruptor ring -> event handlers
//! - Rust: async TLS read loop -> channel -> Disruptor ring -> event handlers
//!
//! The disruptor ring eliminates per-event atomic contention that `tokio::sync::mpsc`
//! incurs. Events are pre-allocated in the ring buffer (zero allocation on the hot
//! path), and the single-producer barrier uses a plain store (no CAS).
//!
//! # Wait Strategy
//!
//! [`AdaptiveWaitStrategy`] implements a three-phase wait inspired by LMAX Disruptor's
//! `PhasedBackoffWaitStrategy` and tuned for FPSS tick intervals (~100us during active
//! trading).

use std::hint;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use disruptor::{build_single_producer, Producer, Sequence};

use super::FpssEvent;

/// Adaptive wait strategy inspired by LMAX Disruptor's `PhasedBackoffWaitStrategy`.
///
/// Three phases:
/// 1. **Spin** -- busy-wait for `spin_iters` iterations (lowest latency, highest CPU)
/// 2. **Yield** -- `thread::yield_now()` for `yield_iters` iterations (moderate)
/// 3. **Hint** -- `hint::spin_loop()` indefinitely (low CPU, still responsive)
///
/// For FPSS real-time market data, we want the spin phase to cover the typical
/// inter-tick interval (~100us during active trading). At ~3ns per spin iteration,
/// 100 spins covers ~300ns -- well within the FPSS tick interval. The yield phase
/// handles brief pauses between bursts, and the hint phase covers idle periods
/// (pre-market, post-market) without burning a full core.
#[derive(Copy, Clone)]
pub struct AdaptiveWaitStrategy {
    spin_iters: u32,
    yield_iters: u32,
}

impl AdaptiveWaitStrategy {
    /// Create a new adaptive wait strategy with custom iteration counts.
    pub fn new(spin_iters: u32, yield_iters: u32) -> Self {
        Self {
            spin_iters,
            yield_iters,
        }
    }

    /// Tuned for FPSS: 100 spins + 10 yields before falling back to spin_loop hint.
    ///
    /// At ~3ns per spin iteration, 100 spins = ~300ns -- well within the typical
    /// FPSS tick interval. This matches the Java terminal's Disruptor configuration
    /// for real-time market data processing.
    pub fn fpss_default() -> Self {
        Self::new(100, 10)
    }
}

impl disruptor::wait_strategies::WaitStrategy for AdaptiveWaitStrategy {
    #[inline]
    fn wait_for(&self, _sequence: Sequence) {
        // Phase 1: Spin (lowest latency)
        for _ in 0..self.spin_iters {
            hint::spin_loop();
        }
        // Phase 2: Yield (moderate)
        for _ in 0..self.yield_iters {
            thread::yield_now();
        }
        // Phase 3: Spin-loop hint (low CPU, still responsive)
        hint::spin_loop();
    }
}

// ---------------------------------------------------------------------------
// Ring event -- the pre-allocated slot in the disruptor ring buffer
// ---------------------------------------------------------------------------

/// FPSS event stored in the disruptor ring buffer.
///
/// Slots are pre-allocated by the ring buffer and reused. The `event` field
/// holds `None` for unused slots and `Some(FpssEvent)` for published events.
///
/// # Why not store FpssEvent directly?
///
/// `FpssEvent` has variants with `Vec<u8>` payloads and `String` fields that
/// cannot be meaningfully pre-allocated. Using `Option<FpssEvent>` lets us
/// pre-allocate the slot metadata while the event data is set on publish.
#[derive(Default)]
pub struct RingEvent {
    /// The FPSS event occupying this slot, or `None` for an empty pre-allocated slot.
    pub event: Option<FpssEvent>,
}

// SAFETY: FpssEvent is Clone + Send; RingEvent is only accessed through the
// disruptor's sequencing guarantees (exclusive write, shared read).
unsafe impl Sync for RingEvent {}

/// Default ring buffer size (must be a power of 2).
///
/// 131072 slots covers ~13 seconds of burst traffic at 10k events/sec with
/// headroom. This matches the Java terminal's `FPSS_QUEUE_DEPTH` spirit while
/// using the disruptor's power-of-2 constraint.
pub const DEFAULT_RING_SIZE: usize = 131_072; // 2^17

/// Minimum ring size (power of 2).
pub const MIN_RING_SIZE: usize = 64;

/// Round up to the next power of 2 (required by disruptor ring buffer).
fn next_power_of_two(n: usize) -> usize {
    if n.is_power_of_two() {
        n
    } else {
        n.next_power_of_two()
    }
}

// ---------------------------------------------------------------------------
// Disruptor pipeline -- bridges async read loop to disruptor to async consumer
// ---------------------------------------------------------------------------

/// Handle for the async read loop to publish events into the disruptor ring.
///
/// This is the "ingress" side of the pipeline. The async read loop calls
/// [`RingPublisher::send`] to push decoded FPSS events into the ring buffer
/// via the feeder thread.
pub struct RingPublisher {
    /// Channel to the feeder thread (bounded, back-pressure aware).
    tx: std_mpsc::SyncSender<FpssEvent>,
}

impl RingPublisher {
    /// Publish an event into the disruptor ring (non-blocking, bounded).
    ///
    /// Returns `Err` if the feeder thread has exited (ring shut down) or
    /// the ingress channel is full (consumer cannot keep up).
    pub fn send(&self, event: FpssEvent) -> Result<(), FpssEvent> {
        self.tx.try_send(event).map_err(|e| match e {
            std_mpsc::TrySendError::Full(ev) | std_mpsc::TrySendError::Disconnected(ev) => ev,
        })
    }
}

/// Opaque handle to the disruptor pipeline.
///
/// Owns the feeder thread. When dropped, signals the feeder to shut down
/// and joins it. The feeder's `SingleProducer` drop then joins the disruptor
/// consumer thread.
pub struct RingPipeline {
    /// Shutdown signal: set to `true` to stop the feeder thread.
    shutdown: Arc<AtomicBool>,
    /// Feeder thread join handle.
    feeder_handle: Option<thread::JoinHandle<()>>,
}

impl Drop for RingPipeline {
    fn drop(&mut self) {
        // Signal the feeder thread to exit its recv_timeout loop.
        self.shutdown.store(true, Ordering::Release);
        if let Some(h) = self.feeder_handle.take() {
            let _ = h.join();
        }
    }
}

/// Build the disruptor pipeline.
///
/// Returns:
/// - `RingPublisher` -- the async read loop uses this to publish events
/// - `RingPipeline`  -- owns the background threads; drop to shut down
///
/// The `event_tx` is the tokio mpsc sender that the disruptor consumer uses
/// to forward events to the async consumer code.
///
/// `ring_size` is rounded up to the next power of 2 as required by the
/// disruptor. If `ring_size < MIN_RING_SIZE`, `MIN_RING_SIZE` is used.
pub fn build_ring_pipeline(
    event_tx: tokio::sync::mpsc::Sender<FpssEvent>,
    ring_size: usize,
) -> (RingPublisher, RingPipeline) {
    let ring_size = next_power_of_two(ring_size.max(MIN_RING_SIZE));

    // Bounded channel from async read loop -> feeder thread.
    // Use a smaller ingress buffer to avoid doubling memory; the ring itself
    // provides the buffering.
    let ingress_depth = (ring_size / 4).max(64);
    let (ingress_tx, ingress_rx) = std_mpsc::sync_channel::<FpssEvent>(ingress_depth);

    let shutdown = Arc::new(AtomicBool::new(false));
    let feeder_shutdown = Arc::clone(&shutdown);

    let feeder_handle = thread::Builder::new()
        .name("fpss-disruptor".to_owned())
        .spawn(move || {
            feeder_loop(ingress_rx, event_tx, ring_size, feeder_shutdown);
        })
        .expect("failed to spawn fpss-disruptor thread");

    let publisher = RingPublisher { tx: ingress_tx };
    let pipeline = RingPipeline {
        shutdown,
        feeder_handle: Some(feeder_handle),
    };

    (publisher, pipeline)
}

/// Feeder thread: reads from the ingress channel, publishes into the disruptor,
/// and the disruptor consumer forwards to the tokio mpsc sender.
///
/// The feeder uses `recv_timeout` (1ms) instead of blocking `recv` so it can
/// check the shutdown flag periodically. This ensures clean shutdown even when
/// the `RingPublisher` is dropped after the `RingPipeline`.
fn feeder_loop(
    ingress_rx: std_mpsc::Receiver<FpssEvent>,
    event_tx: tokio::sync::mpsc::Sender<FpssEvent>,
    ring_size: usize,
    shutdown: Arc<AtomicBool>,
) {
    // Build the disruptor with our adaptive wait strategy.
    let factory = || RingEvent { event: None };
    let wait_strategy = AdaptiveWaitStrategy::fpss_default();

    // The consumer handler forwards events to the tokio channel.
    // We use `try_send` to avoid blocking the disruptor consumer thread.
    // If the async consumer is slow, events are dropped (matching the existing
    // mpsc behavior where sends are fire-and-forget with `let _ = send().await`).
    let mut producer = build_single_producer(ring_size, factory, wait_strategy)
        .handle_events_with(
            move |ring_event: &RingEvent, _sequence: Sequence, _eob: bool| {
                if let Some(ref evt) = ring_event.event {
                    // Clone the event for the tokio channel. The ring slot is reused.
                    let _ = event_tx.try_send(evt.clone());
                }
            },
        )
        .build();

    // Poll interval for checking shutdown flag when no events are available.
    let poll_interval = Duration::from_millis(1);

    // Read from ingress and publish into the ring.
    loop {
        if shutdown.load(Ordering::Acquire) {
            break;
        }

        match ingress_rx.recv_timeout(poll_interval) {
            Ok(event) => {
                producer.publish(|slot| {
                    slot.event = Some(event);
                });
            }
            Err(std_mpsc::RecvTimeoutError::Timeout) => {
                // No event yet -- loop back to check shutdown flag.
                continue;
            }
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                // All senders dropped -- exit.
                break;
            }
        }
    }

    // producer drop will signal shutdown to the disruptor consumer thread
    // and join it before returning.
    tracing::debug!("fpss-disruptor feeder loop exiting");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::enums::RemoveReason;

    #[test]
    fn adaptive_wait_strategy_is_copy_send() {
        fn assert_copy_send<T: Copy + Send>() {}
        assert_copy_send::<AdaptiveWaitStrategy>();
    }

    #[test]
    fn fpss_default_strategy() {
        let s = AdaptiveWaitStrategy::fpss_default();
        assert_eq!(s.spin_iters, 100);
        assert_eq!(s.yield_iters, 10);
    }

    #[test]
    fn ring_event_default_is_none() {
        let e = RingEvent::default();
        assert!(e.event.is_none());
    }

    #[test]
    fn next_power_of_two_already_power() {
        assert_eq!(next_power_of_two(64), 64);
        assert_eq!(next_power_of_two(1024), 1024);
    }

    #[test]
    fn next_power_of_two_rounds_up() {
        assert_eq!(next_power_of_two(65), 128);
        assert_eq!(next_power_of_two(1000), 1024);
        assert_eq!(next_power_of_two(100_000), 131_072);
    }

    #[test]
    fn default_ring_size_is_power_of_two() {
        assert!(DEFAULT_RING_SIZE.is_power_of_two());
    }

    #[tokio::test]
    async fn ring_pipeline_dispatches_events() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);
        let (publisher, _pipeline) = build_ring_pipeline(tx, 64);

        // Publish some events.
        publisher
            .send(FpssEvent::MarketOpen)
            .expect("send should succeed");
        publisher
            .send(FpssEvent::MarketClose)
            .expect("send should succeed");
        publisher
            .send(FpssEvent::ServerError {
                message: "test".to_string(),
            })
            .expect("send should succeed");

        // Give the disruptor threads time to process.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }

        assert_eq!(events.len(), 3);
        assert!(matches!(events[0], FpssEvent::MarketOpen));
        assert!(matches!(events[1], FpssEvent::MarketClose));
        assert!(matches!(events[2], FpssEvent::ServerError { .. }));
    }

    #[tokio::test]
    async fn ring_pipeline_handles_payload_events() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);
        let (publisher, _pipeline) = build_ring_pipeline(tx, 64);

        let payload = vec![1u8, 2, 3, 4, 5];
        publisher
            .send(FpssEvent::QuoteData {
                payload: payload.clone(),
            })
            .expect("send should succeed");
        publisher
            .send(FpssEvent::TradeData {
                payload: payload.clone(),
            })
            .expect("send should succeed");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let mut events = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            events.push(evt);
        }

        assert_eq!(events.len(), 2);
        match &events[0] {
            FpssEvent::QuoteData { payload: p } => assert_eq!(p, &payload),
            other => panic!("expected QuoteData, got {other:?}"),
        }
        match &events[1] {
            FpssEvent::TradeData { payload: p } => assert_eq!(p, &payload),
            other => panic!("expected TradeData, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn ring_pipeline_shutdown_on_publisher_drop() {
        let (tx, _rx) = tokio::sync::mpsc::channel(128);
        let (publisher, pipeline) = build_ring_pipeline(tx, 64);

        // Drop the publisher -- feeder thread should exit.
        drop(publisher);

        // Pipeline drop should join cleanly without hanging.
        drop(pipeline);
    }

    #[tokio::test]
    async fn ring_pipeline_high_throughput() {
        // Use a large ring and ingress buffer so the non-blocking sends don't
        // back-pressure during the burst.
        let (tx, mut rx) = tokio::sync::mpsc::channel(8192);
        let (publisher, _pipeline) = build_ring_pipeline(tx, 4096);

        let count = 1000usize;
        let mut sent = 0usize;
        for i in 0..count {
            if publisher
                .send(FpssEvent::QuoteData {
                    payload: vec![i as u8],
                })
                .is_ok()
            {
                sent += 1;
            }
        }

        // Give the disruptor pipeline time to drain.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        let mut received = 0usize;
        while rx.try_recv().is_ok() {
            received += 1;
        }

        // All successfully-sent events should arrive (the tokio channel is 8192
        // deep, so the consumer never drops). We allow a small margin for timing.
        assert!(
            received >= sent * 9 / 10,
            "expected at least 90% of {sent} sent events, got {received}",
        );
        // Sanity: we should have sent most of the burst.
        assert!(
            sent > count / 2,
            "expected to send at least {}, but only sent {sent}",
            count / 2
        );
    }

    #[test]
    fn ring_size_respects_minimum() {
        // Even if caller requests a tiny ring, we enforce MIN_RING_SIZE.
        let size = next_power_of_two(1_usize.max(MIN_RING_SIZE));
        assert!(size >= MIN_RING_SIZE);
        assert!(size.is_power_of_two());
    }

    #[tokio::test]
    async fn ring_pipeline_disconnect_event() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(128);
        let (publisher, _pipeline) = build_ring_pipeline(tx, 64);

        publisher
            .send(FpssEvent::Disconnected {
                reason: RemoveReason::ServerRestarting,
            })
            .expect("send should succeed");

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let evt = rx.try_recv().expect("should receive event");
        match evt {
            FpssEvent::Disconnected { reason } => {
                assert_eq!(reason, RemoveReason::ServerRestarting);
            }
            other => panic!("expected Disconnected, got {other:?}"),
        }
    }
}
