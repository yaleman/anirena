use qbit_rs::Qbit;
use qbit_rs::model::{AddTorrentArg, Credential, TorrentSource};
use reqwest::{Client, Url};
use serde_json::json;

use crate::cli::CLiOpts;

pub struct QbtClient {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub category: Option<String>,
    pub auto_start: bool,
    pub ratio_limit: Option<f64>,
    pub remove_at_ratio_limit: bool,
}

impl QbtClient {
    pub async fn add_torrents(&self, urls: Vec<Url>) -> Result<(), Box<dyn std::error::Error>> {
        if urls.is_empty() {
            return Ok(());
        }
        let endpoint = Url::parse(&self.base_url)?;
        let client = Client::builder()
            .cookie_store(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        let credential = Credential::new(&self.username, &self.password);
        let api = Qbit::new_with_client(endpoint.clone(), credential, client.clone());
        eprintln!("Connected to QBT API Version: {}", api.get_version().await?);
        if self.remove_at_ratio_limit {
            eprintln!(
                "Setting qBittorrent's global share-limit action to remove torrents and keep files; this also applies to existing torrents."
            );
            // Send only this preference: qbit-rs serializes unset preferences as null.
            // Action 1 removes the torrent without its content.
            client
                .post(endpoint.join("api/v2/")?.join("app/setPreferences")?)
                .form(&[("json", json!({"max_ratio_act": 1}).to_string())])
                .send()
                .await?
                .error_for_status()?;
        }
        api.add_torrent(AddTorrentArg {
            source: TorrentSource::Urls { urls: urls.into() },
            category: self.category.clone(),
            stopped: Some((!self.auto_start).to_string()),
            ratio_limit: self.ratio_limit,
            ..Default::default()
        })
        .await?;
        Ok(())
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
            ratio_limit: opts.qbt_ratio_limit,
            remove_at_ratio_limit: opts.qbt_remove_at_ratio_limit,
        }
    }
}
