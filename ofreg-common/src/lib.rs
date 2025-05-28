pub const SOCK_PATH: &str = "/run/user/1000/ofreg.sock";
pub const TABLE_NAME: &str = "ofreg";
pub const DB_PATH: &str = "/var/db/ofreg";

#[derive(Debug, serde::Serialize, serde::Deserialize, tabled::Tabled)]
pub struct OfregData {
    pub cmd: String,
    pub op_file: String,
    pub time: String,
}
