use procfs::process::Process;
use std::collections::HashMap;

#[derive(Default)]
pub struct SmbdProcessStats {
    pub utime: u64,
    pub stime: u64,
    pub virtual_memory_bytes: u64,
    pub thread_count: u64,
    pub open_fds: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}

pub struct AggregatedStats {
    pub processes: HashMap<i32, SmbdProcessStats>,
    pub total_cpu_time: u64,
    pub total_memory: u64,
    pub total_threads: u64,
    pub total_fds: u64,
    pub total_read: u64,
    pub total_write: u64,
}

pub fn get_process_metrics(pids: &[i32]) -> AggregatedStats {
    let mut agg = AggregatedStats {
        processes: HashMap::new(),
        total_cpu_time: 0,
        total_memory: 0,
        total_threads: 0,
        total_fds: 0,
        total_read: 0,
        total_write: 0,
    };

    for &pid in pids {
        if let Ok(p) = Process::new(pid) {
            let mut stats = SmbdProcessStats::default();

            // 1. CPU and Memory from /proc/[pid]/stat
            if let Ok(stat) = p.stat() {
                stats.utime = stat.utime;
                stats.stime = stat.stime;
                stats.virtual_memory_bytes = stat.vsize;
                stats.thread_count = stat.num_threads as u64;

                agg.total_cpu_time += stat.utime + stat.stime;
                agg.total_memory += stat.vsize;
                agg.total_threads += stat.num_threads as u64;
            }

            // 2. IO from /proc/[pid]/io
            if let Ok(io) = p.io() {
                stats.read_bytes = io.read_bytes;
                stats.write_bytes = io.write_bytes;

                agg.total_read += io.read_bytes;
                agg.total_write += io.write_bytes;
            }

            // 3. FDs (Open Files) from /proc/[pid]/fd
            if let Ok(fds) = p.fd_count() {
                stats.open_fds = fds as u64;
                agg.total_fds += fds as u64;
            }

            agg.processes.insert(pid, stats);
        }
    }
    agg
}
