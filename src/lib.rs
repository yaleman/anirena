#![allow(dead_code)]

use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub mod cli;
pub mod qbt;

pub const API_BASE_URL: &str = "https://www.anirena.com";

pub struct AnirenaClient {
    pub api_key: String,
    token_cache: Option<TokenCache>,
    cache_path: PathBuf,
    client: Client,
}

impl AnirenaClient {
    pub fn new(api_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let cache_path = home::home_dir()
            .map(|home| home.join(".cache/anirena.json"))
            .expect("Failed to determine home directory");
        let token_cache = match load_token_cache(&cache_path) {
            Ok(TokenCacheState::Valid(cache)) => Some(cache),
            Ok(TokenCacheState::Missing | TokenCacheState::Expired) => None,
            Err(error) => {
                eprintln!("Failed to load token cache: {error}");
                None
            }
        };

        Ok(Self {
            api_key: api_key.to_string(),
            token_cache,
            cache_path,
            client: Client::builder()
                .user_agent(format!("anirena-rs/{}", env!("CARGO_PKG_VERSION")))
                .timeout(std::time::Duration::from_secs(30))
                .build()?,
        })
    }

    pub async fn get_token(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(cache) = &self.token_cache
            && !self.token_expired()
        {
            return Ok(cache.bearer_token.clone());
        }

        let response = self
            .client
            .post(format!("{}/api/v1/auth/token", API_BASE_URL))
            .header("Authorization", format!("ApiKey {}", self.api_key))
            .send()
            .await?;
        if response.status().is_success() {
            println!("Successfully retrieved token.");
        } else {
            println!("Failed to retrieve token. Status: {}", response.status());
            return Err(Box::new(io::Error::other("Failed to retrieve token")));
        }

        let token_response: TokenResponse = response.json().await?;
        let cache = TokenCache {
            bearer_token: token_response.token.clone(),
            expires_at: Utc::now()
                + chrono::Duration::seconds(i64::from(token_response.expires_in)),
        };

        if let Err(error) = save_token_cache(&self.cache_path, &cache) {
            eprintln!("Failed to save token cache: {error}");
        }

        self.token_cache = Some(cache);
        Ok(token_response.token)
    }

    pub fn token_expired(&self) -> bool {
        match &self.token_cache {
            Some(cache) => cache.expires_at <= Utc::now(),
            None => true,
        }
    }

    pub async fn search(
        &mut self,
        terms: &[String],
        page: Option<u32>,
        pages: Option<u32>,
    ) -> Result<Vec<Torrent>, Box<dyn std::error::Error>> {
        let token = self.get_token().await?;

        let mut requested_page = page;
        let mut results = Vec::new();
        loop {
            let search_results = do_search(&self.client, &token, terms, requested_page).await?;
            let current_page = search_results.page;
            let total_pages = search_results.total_pages;

            if search_results.torrents.is_empty() {
                println!("No results found for the search terms.");
            } else {
                results.extend(search_results.torrents.clone());
            }

            requested_page = next_search_page(current_page, total_pages, pages);
            if requested_page.is_none() {
                if current_page < total_pages {
                    println!("Note: There are more results available. Total pages: {total_pages}");
                }
                break;
            }
        }
        results.sort_by_key(|a| a.title.to_lowercase());
        Ok(results)
    }
}

fn next_search_page(
    current_page: u32,
    total_pages: u32,
    requested_last_page: Option<u32>,
) -> Option<u32> {
    match requested_last_page {
        Some(last_page) if current_page < last_page && current_page < total_pages => {
            current_page.checked_add(1)
        }
        Some(_) | None => None,
    }
}

async fn do_search(
    http_client: &reqwest::Client,
    token: &str,
    terms: &[String],
    page: Option<u32>,
) -> Result<SearchResults, Box<dyn std::error::Error>> {
    let mut search_payload = HashMap::from([("q", json!(terms.join(" ")))]);
    if let Some(page) = page {
        search_payload.insert("page", json!(page));
    }

    let response = http_client
        .post(format!("{}/api/v1/torrents/search", API_BASE_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&search_payload)
        .send()
        .await?;

    if !response.status().is_success() {
        println!("Failed to perform search. Status: {}", response.status());
        return Err(std::io::Error::other("Failed to perform search").into());
    }

    let search_results: serde_json::Value = response
        .json()
        .await
        .inspect_err(|err| eprintln!("failed to serde::value: {err}"))?;
    serde_json::from_value(search_results.clone())
        .inspect_err(|err| {
            eprintln!("Failed to decode input: {}", json!(search_results));
            eprintln!("Error: {}", err);
        })
        .map_err(|err| Box::new(err) as Box<dyn std::error::Error>)
}

pub fn format_magnet_line(magnet: &str, hyperlinks_enabled: bool) -> String {
    let contains_control_characters = magnet.chars().any(char::is_control);
    let sanitized_magnet = magnet
        .chars()
        .filter(|character| !character.is_control())
        .collect::<String>();

    if hyperlinks_enabled && !contains_control_characters {
        format!("🧲  \x1b]8;;{sanitized_magnet}\x1b\\{sanitized_magnet}\x1b]8;;\x1b\\")
    } else {
        format!("🧲  {sanitized_magnet}")
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TokenCache {
    bearer_token: String,
    expires_at: DateTime<Utc>,
}

enum TokenCacheState {
    Missing,
    Expired,
    Valid(TokenCache),
}

#[derive(Debug)]
enum TokenCacheError {
    Read(io::Error),
    Decode(serde_json::Error),
    CreateDirectory(io::Error),
    Encode(serde_json::Error),
    Write(io::Error),
}

impl fmt::Display for TokenCacheError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(error) => write!(formatter, "could not read cache: {error}"),
            Self::Decode(error) => write!(formatter, "cache contains invalid JSON: {error}"),
            Self::CreateDirectory(error) => {
                write!(formatter, "could not create cache directory: {error}")
            }
            Self::Encode(error) => write!(formatter, "could not encode cache: {error}"),
            Self::Write(error) => write!(formatter, "could not write cache: {error}"),
        }
    }
}

impl std::error::Error for TokenCacheError {}

fn load_token_cache(path: &Path) -> Result<TokenCacheState, TokenCacheError> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(TokenCacheState::Missing);
        }
        Err(error) => return Err(TokenCacheError::Read(error)),
    };
    let cache: TokenCache = serde_json::from_slice(&contents).map_err(TokenCacheError::Decode)?;

    if cache.expires_at <= Utc::now() {
        Ok(TokenCacheState::Expired)
    } else {
        Ok(TokenCacheState::Valid(cache))
    }
}

fn save_token_cache(path: &Path, cache: &TokenCache) -> Result<(), TokenCacheError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(TokenCacheError::CreateDirectory)?;
    }

    let contents = serde_json::to_vec_pretty(cache).map_err(TokenCacheError::Encode)?;
    write_private_file(path, &contents).map_err(TokenCacheError::Write)
}

#[cfg(unix)]
fn write_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.set_permissions(<fs::Permissions as std::os::unix::fs::PermissionsExt>::from_mode(0o600))?;
    file.write_all(contents)
}

#[cfg(not(unix))]
fn write_private_file(path: &Path, contents: &[u8]) -> io::Result<()> {
    fs::write(path, contents)
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct TokenResponse {
    token: String,
    token_type: String,
    expires_in: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct SearchResults {
    total: u32,
    page: u32,
    per_page: u32,
    total_pages: u32,
    from: u32,
    to: u32,
    torrents: Vec<Torrent>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Torrent {
    pub id: uuid::Uuid,
    pub title: String,
    pub info_hash_v1: Option<String>,
    pub info_hash_v2: Option<String>,
    pub size_fmt: String,
    pub completed: u64,
    pub seeders: u32,
    pub leechers: u32,
    pub languages: Vec<String>,
    pub comment_count: u32,
    pub created_at: String,
    pub created_at_unix: u64,
    pub cat_slug: Option<String>,
    pub sub_slug: Option<String>,
    pub group_name: Option<String>,
    pub uploader: Option<String>,
    pub magnet: String,
}

impl Torrent {
    pub fn created_at(&self) -> Option<DateTime<Utc>> {
        DateTime::from_timestamp_secs(self.created_at_unix as i64)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temporary_cache_path(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "anirena-{test_name}-{}-{unique}.json",
            std::process::id()
        ))
    }

    #[test]
    fn token_cache_round_trips() {
        let path = temporary_cache_path("round-trip");
        let cache = TokenCache {
            bearer_token: "cached-token".to_owned(),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        };

        save_token_cache(&path, &cache).expect("token cache should save");
        let loaded = load_token_cache(&path).expect("token cache should load");

        match loaded {
            TokenCacheState::Valid(loaded) => {
                assert_eq!(loaded.bearer_token, cache.bearer_token);
                assert_eq!(loaded.expires_at, cache.expires_at);
            }
            TokenCacheState::Missing | TokenCacheState::Expired => {
                panic!("saved token cache should be valid")
            }
        }

        fs::remove_file(path).expect("temporary token cache should be removed");
    }

    #[test]
    fn expired_token_cache_is_ignored() {
        let path = temporary_cache_path("expired");
        let cache = TokenCache {
            bearer_token: "expired-token".to_owned(),
            expires_at: Utc::now() - chrono::Duration::minutes(5),
        };

        save_token_cache(&path, &cache).expect("token cache should save");
        let loaded = load_token_cache(&path).expect("token cache should load");

        assert!(matches!(loaded, TokenCacheState::Expired));

        fs::remove_file(path).expect("temporary token cache should be removed");
    }

    #[test]
    fn magnet_line_is_clickable_in_interactive_terminals() {
        let magnet = "magnet:?xt=urn:btih:example";

        assert_eq!(
            format_magnet_line(magnet, true),
            "🧲  \x1b]8;;magnet:?xt=urn:btih:example\x1b\\magnet:?xt=urn:btih:example\x1b]8;;\x1b\\"
        );
    }

    #[test]
    fn magnet_line_is_plain_when_output_is_redirected() {
        let magnet = "magnet:?xt=urn:btih:example";

        assert_eq!(
            format_magnet_line(magnet, false),
            "🧲  magnet:?xt=urn:btih:example"
        );
    }

    #[test]
    fn magnet_line_does_not_hyperlink_control_characters() {
        let magnet = "magnet:?xt=urn:btih:example\x1b]8;;https://example.com\n";
        let formatted = format_magnet_line(magnet, true);

        assert_eq!(
            formatted,
            "🧲  magnet:?xt=urn:btih:example]8;;https://example.com"
        );
        assert!(!formatted.contains('\x1b'));
        assert!(!formatted.contains('\n'));
    }

    #[test]
    fn search_continues_until_the_requested_page() {
        assert_eq!(next_search_page(1, 5, Some(3)), Some(2));
        assert_eq!(next_search_page(2, 5, Some(3)), Some(3));
        assert_eq!(next_search_page(3, 5, Some(3)), None);
    }

    #[test]
    fn search_stops_at_the_last_available_page() {
        assert_eq!(next_search_page(2, 3, Some(5)), Some(3));
        assert_eq!(next_search_page(3, 3, Some(5)), None);
    }

    #[test]
    fn search_does_not_continue_without_a_page_limit() {
        assert_eq!(next_search_page(1, 5, None), None);
    }
}
