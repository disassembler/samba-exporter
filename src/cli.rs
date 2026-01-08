use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Samba Prometheus Exporter")]
pub struct Args {
    /// Address to listen on for Prometheus scrapes
    #[arg(short, long, default_value = "0.0.0.0", env = "SAMBA_EXPORTER_ADDRESS")]
    pub listen_address: String,

    /// Port to listen on
    #[arg(short, long, default_value = "9922", env = "SAMBA_EXPORTER_PORT")]
    pub port: u16,

    /// Absolute path to the 'smbstatus' binary
    #[arg(
        long,
        default_value = "smbstatus",
        env = "SAMBA_EXPORTER_SMBSTATUS_PATH"
    )]
    pub smbstatus_path: String,

    /// How long to wait for smbstatus to respond before timing out [ms]
    #[arg(long, default_value = "5000")]
    pub smbstatus_timeout: u64,

    /// Disable per-PID process metrics (CPU/Mem) to save resources on very large servers
    #[arg(long, default_value = "false")]
    pub disable_process_metrics: bool,

    /// Gather metrics in cluster mode (handles node:pid format)
    #[arg(long, default_value = "false")]
    pub cluster_mode: bool,
}
