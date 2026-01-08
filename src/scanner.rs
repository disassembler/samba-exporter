use chrono::NaiveDateTime;
use std::collections::{HashMap, HashSet};
use std::process::Command;

pub struct ParsedSamba {
    pub version: String,
    pub pids: Vec<i32>,
    pub users: HashSet<String>,
    pub protocols: HashMap<String, i64>,
    pub encryption: HashMap<String, i64>,
    pub signing: HashMap<String, i64>,
    pub shares: HashSet<String>,
    pub lock_count: i64,
    pub oldest_connection_unix: Option<i64>,
}

/// Helper to find the index where the "----" table separator begins
fn find_sep_idx(lines: &[&str]) -> Option<usize> {
    lines
        .iter()
        .position(|l| l.starts_with("-----------------"))
}

/// Tries to parse Samba's varied timestamp formats
fn parse_samba_time(time_parts: &[&str]) -> Option<i64> {
    let time_str = time_parts.join(" ");
    if time_str.is_empty() {
        return None;
    }

    let formats = [
        "%a %b %e %H:%M:%S %Y",    // Mon Jan  8 15:00:00 2026
        "%a %b %d %H:%M:%S %Y",    // Mon Jan 08 15:00:00 2026
        "%Y/%m/%d %H:%M:%S",       // 2026/01/08 15:00:00
        "%a %b %e %H:%M:%S %Y %Z", // With Timezone
    ];

    for fmt in formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(&time_str, fmt) {
            return Some(naive.and_utc().timestamp());
        }
    }
    None
}

pub fn get_metrics(bin: &str) -> ParsedSamba {
    let mut metrics = ParsedSamba {
        version: "unknown".to_string(),
        pids: Vec::new(),
        users: HashSet::new(),
        protocols: HashMap::new(),
        encryption: HashMap::new(),
        signing: HashMap::new(),
        shares: HashSet::new(),
        lock_count: 0,
        oldest_connection_unix: None,
    };

    // --- 1. PROCESS DATA (-p -n) ---
    if let Ok(output) = Command::new(bin).args(["-p", "-n"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        if let Some(idx) = find_sep_idx(&lines) {
            // Version extraction (usually 2 lines above separator)
            if idx >= 2 && lines[idx - 2].starts_with("Samba version") {
                metrics.version = lines[idx - 2]
                    .replace("Samba version", "")
                    .trim()
                    .to_string();
            }

            for line in lines.iter().skip(idx + 1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 7 {
                    let pid_part = parts[0].split(':').last().unwrap_or(parts[0]);
                    if let Ok(pid) = pid_part.parse::<i32>() {
                        metrics.pids.push(pid);
                    }
                    metrics.users.insert(parts[1].to_string());
                    *metrics.protocols.entry(parts[4].to_string()).or_insert(0) += 1;
                    *metrics.encryption.entry(parts[5].to_string()).or_insert(0) += 1;
                    *metrics.signing.entry(parts[6].to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    // --- 2. SHARE DATA (-S -n) ---
    if let Ok(output) = Command::new(bin).args(["-S", "-n"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        if let Some(idx) = find_sep_idx(&lines) {
            for line in lines.iter().skip(idx + 1) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    metrics.shares.insert(parts[0].to_string());

                    // Attempt to parse the oldest connection time
                    // Standard -S layout: Service PID Machine ConnectedAt...
                    // ConnectedAt usually starts at parts[3]
                    if parts.len() >= 8 {
                        if let Some(ts) = parse_samba_time(&parts[3..8]) {
                            if metrics.oldest_connection_unix.map_or(true, |old| ts < old) {
                                metrics.oldest_connection_unix = Some(ts);
                            }
                        }
                    }
                }
            }
        }
    }

    // --- 3. LOCK DATA (-L -n) ---
    if let Ok(output) = Command::new(bin).args(["-L", "-n"]).output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();
        if let Some(idx) = find_sep_idx(&lines) {
            metrics.lock_count = lines
                .iter()
                .skip(idx + 1)
                .filter(|l| !l.trim().is_empty())
                .count() as i64;
        }
    }

    metrics
}
