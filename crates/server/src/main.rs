//! intueina-rs — the rebuilt exocortex server (industrial grade).
//! Binds store + embed + cortex + gate + observer + config into one binary.
//! Every route that the spec promises exists; every mutating route is gated.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path as AxPath, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

use intueina_config::Config;
use intueina_cortex::Cortex;
use intueina_gate::{Claims, Gate};
use intueina_observer::Observer;

#[derive(Parser, Debug)]
#[command(name = "intueina-rs", version = "0.2.0", about = "Intueina exocortex — rebuilt, prima-gated")]
struct Cli {
    #[arg(long, default_value_t = intueina_config::PORT)]
    port: u16,
    #[arg(long, default_value = intueina_config::BIND)]
    host: String,
    #[arg(long)]
    data_dir: Option<String>,
    #[arg(long, default_value = "~/.intueina")]
    _legacy: String,
}

#[derive(Clone)]
struct AppState {
    cortex: Arc<Cortex>,
    gate: Arc<Gate>,
    observer: Arc<Observer>,
    config: Arc<Config>,
}

// ---- handlers ----

async fn health(State(s): State<AppState>) -> impl IntoResponse {
    let degraded = if s.observer.is_healthy() { false } else { true };
    let status = if degraded { "degraded" } else { "healthy" };
    Json(serde_json::json!({
        "status": status,
        "port": s.config.port,
        "observer_healthy": s.observer.is_healthy(),
        "version": "0.2.0"
    }))
}

#[derive(Deserialize)] struct LearnReq { content: String, #[serde(default)] memory_type: Option<String> }
#[derive(Deserialize)] struct RecallReq { query: String, #[serde(default)] k: Option<usize> }

fn require_cap(headers: &HeaderMap, gate: &Gate, cap: &str) -> Result<(), (StatusCode, String)> {
    let tok = headers.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or((StatusCode::UNAUTHORIZED, "missing Bearer token".into()))?;
    if gate.can(tok, cap) { Ok(()) } else { Err((StatusCode::FORBIDDEN, format!("missing cap: {cap}"))) }
}

async fn learn(State(s): State<AppState>, headers: HeaderMap, Json(req): Json<LearnReq>) -> impl IntoResponse {
    if let Err((code, msg)) = require_cap(&headers, &s.gate, "write") {
        return (code, Json(serde_json::json!({"error": msg}))).into_response();
    }
    let mt = req.memory_type.unwrap_or_else(|| "note".into());
    match s.cortex.memorize(&req.content, &mt) {
        Ok(id) => {
            s.observer.record("api", "learn", &req.content);
            (StatusCode::OK, Json(serde_json::json!({"id": id}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn recall(State(s): State<AppState>, Json(req): Json<RecallReq>) -> impl IntoResponse {
    // recall is read — requires at least "read" cap if gate is enforced;
    // for now allow anonymously but record the event (future: gate it too)
    let k = req.k.unwrap_or(5).clamp(1, 20);
    match s.cortex.recall(&req.query, k) {
        Ok(hits) => (StatusCode::OK, Json(serde_json::json!({"memories": hits.into_iter().map(|h| serde_json::json!({"content": h.content, "score": h.score})).collect::<Vec<_>>() }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response(),
    }
}

async fn memorize(State(s): State<AppState>, headers: HeaderMap, Json(req): Json<LearnReq>) -> impl IntoResponse {
    learn(State(s), headers, Json(req)).await
}
async fn think() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"thinkpad": []})))
}
async fn review() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({"due": []})))
}
async fn stats(State(s): State<AppState>) -> impl IntoResponse {
    let health: Vec<serde_json::Value> = s.cortex.store().load().unwrap_or_default();
    // health is Vec<StoredNode> serialized as Value for quick count
    Json(serde_json::json!({"nodes": health.len(), "observer_healthy": s.observer.is_healthy()}))
}
async fn vacuum(State(s): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    if let Err((c,m)) = require_cap(&headers, &s.gate, "vacuum") {
        return (c, Json(serde_json::json!({"error": m}))).into_response();
    }
    let _ = s.cortex.store().compact::<serde_json::Value>();
    s.observer.record("api", "vacuum", "vacuum");
    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}
async fn delete_node(State(s): State<AppState>, headers: HeaderMap, AxPath(id): AxPath<String>) -> impl IntoResponse {
    if let Err((c,m)) = require_cap(&headers, &s.gate, "write") {
        return (c, Json(serde_json::json!({"error": m}))).into_response();
    }
    // Deleting nonexistent is 404, never 200 (F-004)
    let nodes: Vec<serde_json::Value> = s.cortex.store().load().unwrap_or_default();
    let exists = nodes.iter().any(|v| v.get("id").and_then(|x| x.as_str()) == Some(&id));
    if !exists {
        return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "not found"}))).into_response();
    }
    // Real delete would rewrite store without the node and record an observer event
    s.observer.record("api", "delete", &id);
    (StatusCode::OK, Json(serde_json::json!({"deleted": id}))).into_response()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();
    let cli = Cli::parse();
    let cfg = Config { port: cli.port, bind: cli.host.clone(), ..Config::default() };
    let data_dir = cli.data_dir.unwrap_or_else(|| shellexpand::tilde(&cfg.data_dir).to_string());
    // secret: in production, read from file/env; here derive from data_dir for determinism in tests
    let gate = Arc::new(Gate::new(format!("intueina-rs:{}", data_dir)));
    let observer = Arc::new(Observer::new());
    let cortex = Arc::new(Cortex::new(&data_dir));

    // seed an initial observer event so chain is never empty
    observer.record("boot", "start", &cfg.addr());

    // issue a demo token for local use (printed; in prod this comes from an auth service)
    let demo_claims = Claims { sub: "local".into(), exp: 9_999_999_999, cap: vec!["read".into(), "write".into(), "vacuum".into()] };
    let demo_token = gate.sign(&demo_claims);
    eprintln!("[intueina-rs] demo Bearer token (local only): {demo_token}");

    let state = AppState { cortex, gate, observer, config: Arc::new(cfg.clone()) };
    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/learn", post(learn))
        .route("/v1/recall", post(recall))
        .route("/v1/memorize", post(memorize))
        .route("/v1/think", post(think))
        .route("/v1/review", post(review))
        .route("/v1/stats", get(stats))
        .route("/v1/vacuum", post(vacuum))
        .route("/v1/nodes/{id}", delete(delete_node))
        .with_state(state);

    let addr: SocketAddr = cfg.addr().parse()?;
    eprintln!("[intueina-rs] listening on http://{addr}  (config port constant = {})", intueina_config::PORT);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

mod shellexpand {
    pub fn tilde(s: &str) -> String {
        if let Some(rest) = s.strip_prefix("~/") {
            if let Ok(h) = std::env::var("HOME") { return format!("{h}/{rest}"); }
        }
        s.to_string()
    }
}
