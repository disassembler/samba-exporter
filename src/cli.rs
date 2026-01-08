use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about = "Samba Prometheus Exporter")]
pub struct Args {
    /// Address to listen on
    #[arg(short, long, default_value = "0.0.0.0")]
    pub listen_address: String,

    /// Port to listen on
    #[arg(short, long, default_value = "9922")]
    pub port: u16,

    /// Path to sessionid.tdb
    #[arg(long, default_value = "/var/lib/samba/sessionid.tdb")]
    pub session_tdb: String,

    /// Path to locking.tdb
    #[arg(long, default_value = "/var/lib/samba/locking.tdb")]
    pub locking_tdb: String,
}
