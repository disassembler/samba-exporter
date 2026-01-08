use axum::{routing::get, Router};
use lazy_static::lazy_static;
use prometheus::{register_int_gauge, Encoder, IntGauge, TextEncoder};
use std::path::Path;
// Based on your doc: we need Flags and Tdb.
// O_RDONLY is usually from libc or constant in the crate.
use trivialdb::{Flags, Tdb, O_RDONLY};

lazy_static! {
    static ref ACTIVE_SESSIONS: IntGauge =
        register_int_gauge!("samba_active_sessions", "Number of active Samba sessions").unwrap();
    static ref FILE_LOCKS: IntGauge =
        register_int_gauge!("samba_file_locks", "Number of active file locks").unwrap();
}

fn scrape_tdb_count(path_str: &str) -> i64 {
    let path = Path::new(path_str);
    if !path.exists() {
        return 0;
    }

    // According to your docs:
    // pub fn open<P: AsRef<Path>>(name: P, hash_size: Option<u32>, tdb_flags: Flags, open_flags: i32, mode: c_uint) -> Option<Tdb>
    match Tdb::open(path, None, Flags::default(), O_RDONLY, 0) {
        Some(tdb) => {
            // Using the Iterator implementation from your docs:
            // pub fn iter(&self) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + '_
            tdb.iter().count() as i64
        }
        None => {
            eprintln!("Could not open TDB file: {}", path_str);
            0
        }
    }
}

async fn metrics_handler() -> String {
    ACTIVE_SESSIONS.set(scrape_tdb_count("/var/lib/samba/sessionid.tdb"));
    FILE_LOCKS.set(scrape_tdb_count("/var/lib/samba/locking.tdb"));

    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/metrics", get(metrics_handler));
    let addr = "0.0.0.0:9922";

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("Samba Exporter running on http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}
