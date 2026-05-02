//! HTTP listener that receives GSI POST payloads from CS2.
//!
//! `GameStateListener` is the single entry point of the library.
//!
//! ```no_run
//! use cs2_gsi::{events::PlayerDied, GameStateListener};
//!
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let listener = GameStateListener::new(4000);
//! listener.on(|e: &PlayerDied| {
//!     println!("{} died at {} HP", e.player.name, e.previous_health);
//! });
//! listener.start().await?;
//! # Ok(()) }
//! ```

use crate::dispatcher::Dispatcher;
use crate::error::{Error, Result};
use crate::events::GameEvent;
use crate::handlers::diff_and_dispatch;
use crate::model::GameState;

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use parking_lot::RwLock;
use std::any::Any;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tracing::{debug, error, instrument, trace, warn};

/// How long to wait between bind retries when the address is reported as
/// in use. Tuned for the typical TIME_WAIT / dev-loop hand-off window —
/// long enough to outlast a parent `cargo tauri dev` rebuild but short
/// enough that a real port conflict surfaces in well under two seconds.
const BIND_RETRY_DELAY: Duration = Duration::from_millis(250);
const BIND_RETRY_ATTEMPTS: usize = 6;

/// HTTP listener for CS2 GSI payloads.
///
/// Cheap to clone — every clone shares the same dispatcher and last-state
/// cache. Handlers registered through any clone fire on every payload.
#[derive(Clone)]
pub struct GameStateListener {
    addr: SocketAddr,
    dispatcher: Dispatcher,
    last_state: Arc<RwLock<Option<GameState>>>,
    runtime: Arc<RwLock<RuntimeHandle>>,
}

#[derive(Default)]
struct RuntimeHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
    join: Option<JoinHandle<Result<()>>>,
    bound_addr: Option<SocketAddr>,
}

impl GameStateListener {
    /// Create a listener that will bind to `127.0.0.1:<port>` when started.
    pub fn new(port: u16) -> Self {
        Self::with_addr(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
    }

    /// Create a listener with a fully specified bind address.
    pub fn with_addr(addr: SocketAddr) -> Self {
        Self {
            addr,
            dispatcher: Dispatcher::default(),
            last_state: Arc::new(RwLock::new(None)),
            runtime: Arc::new(RwLock::new(RuntimeHandle::default())),
        }
    }

    /// The configured bind address. After [`start`](Self::start) succeeds,
    /// this is also returned by [`actual_addr`](Self::actual_addr).
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The actual bound socket address — useful when you bind to port `0`
    /// and want to discover the OS-assigned port.
    pub fn actual_addr(&self) -> Option<SocketAddr> {
        self.runtime.read().bound_addr
    }

    /// Subscribe to a strongly typed event.
    ///
    /// ```no_run
    /// # use cs2_gsi::{events::PlayerGotKill, GameStateListener};
    /// let gsl = GameStateListener::new(4000);
    /// gsl.on(|e: &PlayerGotKill| {
    ///     println!("{} now has {} round kills", e.player.name, e.new_round_kills);
    /// });
    /// ```
    pub fn on<E, F>(&self, handler: F) -> &Self
    where
        E: Any + Send + Sync + 'static,
        F: Fn(&E) + Send + Sync + 'static,
    {
        self.dispatcher.register(handler);
        self
    }

    /// Subscribe to every event as a [`GameEvent`].
    pub fn on_any<F>(&self, handler: F) -> &Self
    where
        F: Fn(&GameEvent) + Send + Sync + 'static,
    {
        self.dispatcher.register_any(handler);
        self
    }

    /// The most recently received game state, if any.
    pub fn current_state(&self) -> Option<GameState> {
        self.last_state.read().clone()
    }

    /// `true` once [`start`](Self::start) has succeeded and before
    /// [`stop`](Self::stop) is called.
    pub fn is_running(&self) -> bool {
        self.runtime.read().shutdown_tx.is_some()
    }

    /// Bind the listener and start accepting GSI payloads. Returns once the
    /// socket is bound — the actual serve loop runs as a background tokio
    /// task. Call [`stop`](Self::stop) to shut it down.
    #[instrument(level = "debug", skip(self), fields(addr = %self.addr))]
    pub async fn start(&self) -> Result<()> {
        if self.is_running() {
            return Err(Error::AlreadyStarted);
        }

        let tcp = bind_with_retry(self.addr).await.map_err(|e| Error::Bind {
            addr: self.addr.to_string(),
            source: e,
        })?;
        let bound = tcp.local_addr()?;

        let dispatcher = self.dispatcher.clone();
        let last_state = self.last_state.clone();
        let (tx, rx) = oneshot::channel::<()>();

        let join = tokio::spawn(serve_loop(tcp, dispatcher, last_state, rx));

        let mut rt = self.runtime.write();
        rt.shutdown_tx = Some(tx);
        rt.join = Some(join);
        rt.bound_addr = Some(bound);
        debug!("GSI listener bound at {bound}");
        Ok(())
    }

    /// Signal the serve loop to exit and wait for it to finish.
    pub async fn stop(&self) -> Result<()> {
        let (tx, join) = {
            let mut rt = self.runtime.write();
            (rt.shutdown_tx.take(), rt.join.take())
        };
        let tx = tx.ok_or(Error::NotRunning)?;
        let _ = tx.send(());
        if let Some(handle) = join {
            match handle.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(join_err) => {
                    warn!("listener join error: {join_err}");
                }
            }
        }
        self.runtime.write().bound_addr = None;
        Ok(())
    }
}

/// Bind to `addr`, retrying briefly on `AddrInUse`.
///
/// Targets the *real* failure mode in dev: when a watcher (cargo tauri
/// dev, cargo-watch, …) restarts the process, the previous binary's
/// socket is usually still in TIME_WAIT for a fraction of a second and
/// the new bind would otherwise return `WSAEADDRINUSE` (Windows) /
/// `EADDRINUSE` (Linux/macOS). Retries are bounded — a genuine port
/// conflict still surfaces in ~1.5s with the original error.
async fn bind_with_retry(addr: SocketAddr) -> io::Result<TcpListener> {
    let mut last_err: Option<io::Error> = None;
    for attempt in 0..BIND_RETRY_ATTEMPTS {
        match TcpListener::bind(addr).await {
            Ok(tcp) => return Ok(tcp),
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                debug!(
                    "bind {addr} returned AddrInUse (attempt {}/{}), retrying in {:?}",
                    attempt + 1,
                    BIND_RETRY_ATTEMPTS,
                    BIND_RETRY_DELAY
                );
                last_err = Some(e);
                tokio::time::sleep(BIND_RETRY_DELAY).await;
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap_or_else(|| io::Error::other("bind retry exhausted")))
}

#[instrument(level = "debug", skip_all, fields(addr = %tcp.local_addr().map(|a| a.to_string()).unwrap_or_default()))]
async fn serve_loop(
    tcp: TcpListener,
    dispatcher: Dispatcher,
    last_state: Arc<RwLock<Option<GameState>>>,
    mut shutdown: oneshot::Receiver<()>,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = &mut shutdown => {
                debug!("shutdown signal received");
                return Ok(());
            }
            accepted = tcp.accept() => {
                let (stream, peer) = match accepted {
                    Ok(p) => p,
                    Err(e) => {
                        error!("accept error: {e}");
                        continue;
                    }
                };
                trace!("connection from {peer}");
                let dispatcher = dispatcher.clone();
                let last_state = last_state.clone();
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req| {
                        let dispatcher = dispatcher.clone();
                        let last_state = last_state.clone();
                        async move { handle_request(req, dispatcher, last_state).await }
                    });
                    if let Err(e) = http1::Builder::new()
                        .keep_alive(true)
                        .serve_connection(io, svc)
                        .await
                    {
                        // CS2 occasionally drops the connection mid-keepalive
                        // — log at trace level so it doesn't spam.
                        trace!("connection {peer} closed: {e}");
                    }
                });
            }
        }
    }
}

async fn handle_request(
    req: Request<Incoming>,
    dispatcher: Dispatcher,
    last_state: Arc<RwLock<Option<GameState>>>,
) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() != Method::POST {
        return Ok(reply(
            StatusCode::METHOD_NOT_ALLOWED,
            "only POST is supported",
        ));
    }

    let body = match req.into_body().collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            warn!("body collect error: {e}");
            return Ok(reply(StatusCode::BAD_REQUEST, "could not read body"));
        }
    };

    match GameState::from_slice(&body) {
        Ok(state) => {
            let prev = {
                let mut guard = last_state.write();
                let prev = guard.clone();
                *guard = Some(state.clone());
                prev
            };
            // Diff & dispatch synchronously — keep deterministic ordering.
            diff_and_dispatch(prev.as_ref(), &state, &dispatcher);
            Ok(reply(StatusCode::OK, ""))
        }
        Err(e) => {
            warn!("invalid GSI payload: {e}");
            Ok(reply(StatusCode::BAD_REQUEST, "invalid payload"))
        }
    }
}

fn reply(status: StatusCode, body: &'static str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("content-type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from_static(body.as_bytes())))
        .expect("static response builder cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::PlayerDied;
    use std::net::SocketAddr;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn payload_with_health(name: &str, hp: i32) -> String {
        format!(
            r#"{{"provider":{{"name":"Counter-Strike 2","appid":"730","version":"14000","steamid":"7656"}},"player":{{"steamid":"7656","name":"{name}","team":"CT","activity":"playing","state":{{"health":"{hp}","armor":"100","money":"800","round_kills":"0","round_killhs":"0","round_totaldmg":"0","equip_value":"800","flashed":"0","smoked":"0","burning":"0"}}}}}}"#
        )
    }

    #[tokio::test]
    async fn end_to_end_player_died() {
        let listener = GameStateListener::with_addr(SocketAddr::from(([127, 0, 0, 1], 0)));
        let died = Arc::new(AtomicUsize::new(0));
        let died2 = died.clone();
        listener.on(move |_e: &PlayerDied| {
            died2.fetch_add(1, Ordering::SeqCst);
        });
        listener.start().await.unwrap();
        let url = format!("http://{}/", listener.actual_addr().unwrap());
        let client = reqwest::Client::new();
        client
            .post(&url)
            .body(payload_with_health("alice", 100))
            .send()
            .await
            .unwrap();
        client
            .post(&url)
            .body(payload_with_health("alice", 0))
            .send()
            .await
            .unwrap();
        // Allow the spawned diff to run.
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(died.load(Ordering::SeqCst), 1);
        listener.stop().await.unwrap();
        assert!(!listener.is_running());
    }

    #[tokio::test]
    async fn rejects_non_post() {
        let listener = GameStateListener::with_addr(SocketAddr::from(([127, 0, 0, 1], 0)));
        listener.start().await.unwrap();
        let url = format!("http://{}/", listener.actual_addr().unwrap());
        let resp = reqwest::Client::new().get(&url).send().await.unwrap();
        assert_eq!(resp.status().as_u16(), 405);
        listener.stop().await.unwrap();
    }

    #[tokio::test]
    async fn bind_retry_succeeds_when_squatter_releases() {
        // Pin a port by binding briefly, releasing it, and pinning the
        // *same* port — emulating the dev-restart TIME_WAIT window.
        let probe = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let addr = probe.local_addr().unwrap();

        // Start a task that holds the port for ~150ms then drops it,
        // well within the retry budget (6 × 250ms = 1.5s).
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(150)).await;
            drop(probe);
        });

        let listener = GameStateListener::with_addr(addr);
        // Without retry, this would race and frequently fail; with
        // retry, the squatter releases on attempt 1 or 2 and the bind
        // succeeds.
        listener.start().await.expect("retry should win the race");
        listener.stop().await.unwrap();
    }

    #[tokio::test]
    async fn bind_retry_eventually_surfaces_real_conflict() {
        // A held port that *never* releases must surface as Bind error
        // within the retry budget — not hang forever.
        let squatter = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .unwrap();
        let addr = squatter.local_addr().unwrap();

        let listener = GameStateListener::with_addr(addr);
        let start = std::time::Instant::now();
        let err = listener.start().await.expect_err("must fail");
        let elapsed = start.elapsed();
        // Total budget is 6 × 250ms = 1.5s; allow some slack.
        assert!(
            elapsed < Duration::from_secs(3),
            "bind retry should bound failure latency, took {elapsed:?}"
        );
        match err {
            Error::Bind { .. } => {}
            other => panic!("expected Bind error, got {other:?}"),
        }
        drop(squatter);
    }
}
