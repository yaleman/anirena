use qbit_rs::Qbit;
use qbit_rs::model::Credential;

use crate::cli::CLiOpts;

pub struct QbtClient {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub category: Option<String>,
    pub auto_start: bool,
}

impl QbtClient {
    pub async fn login(&self) -> Result<Qbit, Box<dyn std::error::Error>> {
        let credential = Credential::new(&self.username, &self.password);
        let api = Qbit::new(self.base_url.as_str(), credential);
        eprintln!("Connected to QBT API Version: {}", api.get_version().await?);

        Ok(api)
    }
}

impl From<&CLiOpts> for QbtClient {
    fn from(opts: &CLiOpts) -> Self {
        QbtClient {
            base_url: opts.qbt_base_url.clone(),
            username: opts.qbt_username.clone(),
            password: opts.qbt_password.clone(),
            category: opts.qbt_category.clone(),
            auto_start: opts.qbt_auto_start,
        }
    }
}
