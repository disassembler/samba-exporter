mod cli;
mod process;
mod scanner;

use axum::{extract::State, routing::get, Router};
use chrono::Utc;
use clap::Parser;
use lazy_static::lazy_static;
use prometheus::{
    register_gauge, register_int_gauge, register_int_gauge_vec, Encoder, Gauge, IntGauge,
    IntGaugeVec, TextEncoder,
};
use std::sync::Arc;

lazy_static! {
    // 1. Metadata & Status
    static ref SERVER_INFO: IntGaugeVec = register_int_gauge_vec!("samba_server_information", "Samba version", &["version"]).unwrap();
    static ref EXPORTER_INFO: IntGaugeVec = register_int_gauge_vec!("samba_exporter_information", "Exporter version", &["version"]).unwrap();
    static ref SERVER_UP: IntGauge = register_int_gauge!("samba_server_up", "1 if samba seems running").unwrap();
    static ref STATUS_UP: IntGauge = register_int_gauge!("samba_statusd_up", "1 if exporter is healthy").unwrap();
    static ref REQUEST_TIME: IntGauge = register_int_gauge!("samba_request_time", "Scrape duration in ms").unwrap();

    // 2. Client & User Metrics
    static ref CLIENT_COUNT: IntGauge = register_int_gauge!("samba_client_count", "Number of connected clients").unwrap();
    static ref USER_COUNT: IntGauge = register_int_gauge!("samba_individual_user_count", "Unique users").unwrap();
    static ref SHARE_COUNT: IntGauge = register_int_gauge!("samba_share_count", "Number of shares served").unwrap();
    static ref PID_COUNT: IntGauge = register_int_gauge!("samba_pid_count", "Number of smbd processes").unwrap();

    // 3. Connection Timings
    static ref CONNECTED_AT: IntGauge = register_int_gauge!("samba_client_connected_at", "Earliest connection timestamp").unwrap();
    static ref CONNECTED_SINCE: IntGauge = register_int_gauge!("samba_client_connected_since_seconds", "Seconds since earliest connection").unwrap();

    // 4. Grouped Stats (Labels)
    static ref PROTOCOL_VERSION: IntGaugeVec = register_int_gauge_vec!("samba_protocol_version_count", "Protocols", &["version"]).unwrap();
    static ref ENCRYPTION_METHOD: IntGaugeVec = register_int_gauge_vec!("samba_encryption_method_count", "Encryption", &["method"]).unwrap();
    static ref SIGNING_METHOD: IntGaugeVec = register_int_gauge_vec!("samba_signing_method_count", "Signing", &["method"]).unwrap();

    // 5. Locking Metrics
    static ref LOCKED_FILES: IntGauge = register_int_gauge!("samba_locked_file_count", "Total locked files").unwrap();
    static ref LOCK_CREATED_AT: IntGauge = register_int_gauge!("samba_lock_created_at", "Oldest lock timestamp").unwrap();

    // 6. Process Metrics (Sums)
    static ref SUM_CPU: Gauge = register_gauge!("samba_smbd_sum_cpu_usage_percentage", "Sum CPU % of all smbd").unwrap();
    static ref SUM_MEM: IntGauge = register_int_gauge!("samba_smbd_sum_virtual_memory_usage_bytes", "Sum VM bytes").unwrap();
    static ref SUM_THREADS: IntGauge = register_int_gauge!("samba_smbd_sum_thread_count", "Sum threads").unwrap();
    static ref SUM_IO_READ: IntGauge = register_int_gauge!("samba_smbd_sum_io_counter_read_bytes", "Sum Read bytes").unwrap();
    static ref SUM_IO_WRITE: IntGauge = register_int_gauge!("samba_smbd_sum_io_counter_write_bytes", "Sum Write bytes").unwrap();
    static ref SUM_FDS: IntGauge = register_int_gauge!("samba_smbd_sum_open_file_count", "Sum open file handles").unwrap();

    // 7. Individual Process Metrics (Labels by PID)
    static ref PID_MEM: IntGaugeVec = register_int_gauge_vec!("samba_smbd_virtual_memory_usage_bytes", "VM per PID", &["pid"]).unwrap();
    static ref PID_FDS: IntGaugeVec = register_int_gauge_vec!("samba_smbd_open_file_count", "Open files per PID", &["pid"]).unwrap();
}

struct AppState {
    args: cli::Args,
}

async fn metrics_handler(State(state): State<Arc<AppState>>) -> String {
    let start_scrape = std::time::Instant::now();
    let now_unix = Utc::now().timestamp();

    // 1. Scrape smbstatus
    let smb = scanner::get_metrics(&state.args.smbstatus_path);

    // Update basic statuses
    SERVER_UP.set(if smb.pids.is_empty() { 0 } else { 1 });
    STATUS_UP.set(1);
    SERVER_INFO.reset();
    SERVER_INFO.with_label_values(&[&smb.version]).set(1);

    // Update Counts
    CLIENT_COUNT.set(smb.pids.len() as i64);
    USER_COUNT.set(smb.users.len() as i64);
    SHARE_COUNT.set(smb.shares.len() as i64);
    LOCKED_FILES.set(smb.lock_count);

    // Cluster mode logic for pid_count
    if !state.args.cluster_mode {
        PID_COUNT.set(smb.pids.len() as i64);
    }

    // Timings
    if let Some(ts) = smb.oldest_connection_unix {
        CONNECTED_AT.set(ts);
        CONNECTED_SINCE.set(now_unix - ts);
    }

    // Reset and fill vectors
    PROTOCOL_VERSION.reset();
    for (v, c) in smb.protocols {
        PROTOCOL_VERSION.with_label_values(&[&v]).set(c);
    }

    ENCRYPTION_METHOD.reset();
    for (m, c) in smb.encryption {
        ENCRYPTION_METHOD.with_label_values(&[&m]).set(c);
    }

    SIGNING_METHOD.reset();
    for (m, c) in smb.signing {
        SIGNING_METHOD.with_label_values(&[&m]).set(c);
    }

    // 2. Scrape Process Data if not disabled
    if !state.args.disable_process_metrics {
        let proc_stats = process::get_process_metrics(&smb.pids);

        SUM_MEM.set(proc_stats.total_memory as i64);
        SUM_THREADS.set(proc_stats.total_threads as i64);
        SUM_FDS.set(proc_stats.total_fds as i64);
        SUM_IO_READ.set(proc_stats.total_read as i64);
        SUM_IO_WRITE.set(proc_stats.total_write as i64);

        // Individual PID labels
        PID_MEM.reset();
        PID_FDS.reset();
        for (pid, s) in proc_stats.processes {
            let pstr = pid.to_string();
            PID_MEM
                .with_label_values(&[&pstr])
                .set(s.virtual_memory_bytes as i64);
            PID_FDS.with_label_values(&[&pstr]).set(s.open_fds as i64);
        }
    }

    // 3. Finalize Scrape
    REQUEST_TIME.set(start_scrape.elapsed().as_millis() as i64);

    let encoder = TextEncoder::new();
    let mut buffer = Vec::new();
    encoder.encode(&prometheus::gather(), &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap_or_default()
}

#[tokio::main]
async fn main() {
    let args = cli::Args::parse();

    // Set static exporter info
    EXPORTER_INFO
        .with_label_values(&[env!("CARGO_PKG_VERSION")])
        .set(1);

    let state = Arc::new(AppState { args: args.clone() });
    let app = Router::new()
        .route("/metrics", get(metrics_handler))
        .with_state(state);

    let addr = format!("{}:{}", args.listen_address, args.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind port");

    println!(
        "Samba Exporter v{} listening on http://{}",
        env!("CARGO_PKG_VERSION"),
        addr
    );
    axum::serve(listener, app).await.unwrap();
}
