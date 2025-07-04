pub const SOCK_PATH: &str = "/run/user/1000/ofreg.sock";
pub const TABLE_NAME: &str = "ofreg";
pub const DB_PATH: &str = "/var/db/ofreg";

#[derive(Debug, serde::Serialize, serde::Deserialize, tabled::Tabled)]
pub struct OfregData {
    pub cmd: String,
    pub op_file: String,
    pub time: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, clap::Parser)]
pub struct Query {
    #[arg(short)]
    pub cmd: Option<String>,
    #[arg(short)]
    pub file: Option<String>,
    #[arg(short = 'b')]
    pub time_begin: Option<u64>,
    #[arg(short = 'e')]
    pub time_end: Option<u64>,
    #[arg(short, default_value_t = 10)]
    pub num: u32,
}
