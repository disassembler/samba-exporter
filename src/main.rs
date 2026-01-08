mod cli;
mod scanner;

use axum::{extract::State, routing::get, Router};
use clap::Parser;
use lazy_static::lazy_static;
use prometheus::{register_int_gauge, Encoder, IntGauge, TextEncoder};
use std::sync::Arc;

lazy_static! {
    static ref ACTIVE_SESSIONS: IntGauge =
        register_int_gauge!("samba_active_sessions", "Active sessions").unwrap();
    static ref FILE_LOCKS: IntGauge =
        register_int_gauge!("samba_file_locks", "Active file locks").unwrap();
}

struct AppState {
    args: cli::Args,
}

async fn metrics_handler(State(state): State<Arc<AppState>>) -> String {
    ACTIVE_SESSIONS.set(scanner::count_records(&state.args.session_tdb));
    FILE_LOCKS.set(scanner::count_records(&state.args.locking_tdb));

    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();
    let state = Arc::new(AppState { args });

    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state.clone());

    let addr = format!("{}:{}", state.args.listen_address, state.args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("Samba Exporter listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
