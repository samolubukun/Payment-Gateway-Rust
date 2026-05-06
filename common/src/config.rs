use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct DbConfig {
    pub database_host: String,
    pub database_port: u16,
    pub database_name: String,
    pub database_user: String,
    pub database_password: String,
}

impl DbConfig {
    pub fn url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.database_user,
            self.database_password,
            self.database_host,
            self.database_port,
            self.database_name,
        )
    }
}
