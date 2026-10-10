use std::collections::HashMap;

use anirena::{cli::CLiOpts, qbt::QbtClient};
use clap::Parser;
use reqwest::Url;
use serde_json::json;
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{method, path},
};

fn options(base_url: &str, extra: &[&str]) -> Result<CLiOpts, clap::Error> {
    CLiOpts::try_parse_from(
        [
            "anirena",
            "--api-key",
            "test",
            "--qbt-base-url",
            base_url,
            "--qbt-username",
            "test",
            "--qbt-password",
            "test",
            "--qbt-category",
            "anime",
        ]
        .into_iter()
        .chain(extra.iter().copied()),
    )
}

async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/auth/login"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("set-cookie", "SID=test; Path=/; HttpOnly")
                .set_body_string("Ok."),
        )
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v2/app/version"))
        .respond_with(ResponseTemplate::new(200).set_body_string("v5.0.3"))
        .mount(&server)
        .await;
    server
}

fn form(request: &Request) -> HashMap<String, String> {
    let body = std::str::from_utf8(&request.body).expect("form should be UTF-8");
    let mut url = Url::parse("http://localhost/").expect("valid URL");
    url.set_query(Some(body));
    url.query_pairs().into_owned().collect()
}

fn magnets() -> Vec<Url> {
    vec![
        Url::parse("magnet:?xt=urn:btih:0123456789012345678901234567890123456789")
            .expect("valid magnet"),
    ]
}

#[tokio::test]
async fn adds_ratio_five_and_sets_torrent_only_removal() {
    let server = server().await;
    for endpoint in ["app/setPreferences", "torrents/add"] {
        Mock::given(method("POST"))
            .and(path(format!("/api/v2/{endpoint}")))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;
    }
    let opts = options(
        &server.uri(),
        &["--qbt-ratio-limit", "5", "--qbt-remove-at-ratio-limit"],
    )
    .expect("valid options");
    QbtClient::from(&opts)
        .add_torrents(magnets())
        .await
        .expect("add should succeed");

    let requests = server
        .received_requests()
        .await
        .expect("requests available");
    let preferences_index = requests
        .iter()
        .position(|r| r.url.path() == "/api/v2/app/setPreferences")
        .expect("preferences request");
    let add_index = requests
        .iter()
        .position(|r| r.url.path() == "/api/v2/torrents/add")
        .expect("add request");
    assert!(preferences_index < add_index);
    assert_eq!(
        requests[preferences_index]
            .headers
            .get("cookie")
            .expect("authenticated preference request"),
        "SID=test"
    );
    let preferences: serde_json::Value =
        serde_json::from_str(&form(&requests[preferences_index])["json"])
            .expect("preferences JSON");
    assert_eq!(preferences, json!({"max_ratio_act": 1}));
    let added = form(&requests[add_index]);
    assert_eq!(
        added["ratioLimit"].parse::<f64>().expect("numeric ratio"),
        5.0
    );
    assert_eq!(added["category"], "anime");
    assert_eq!(added["stopped"], (!opts.qbt_auto_start).to_string());
    assert_eq!(added["urls"], magnets()[0].as_str());
}

#[tokio::test]
async fn default_add_does_not_change_share_settings() {
    let server = server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/torrents/add"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&server)
        .await;
    let opts = options(&server.uri(), &["--qbt-auto-start"]).expect("valid options");
    QbtClient::from(&opts)
        .add_torrents(magnets())
        .await
        .expect("add should succeed");
    let requests = server
        .received_requests()
        .await
        .expect("requests available");
    assert!(
        !requests
            .iter()
            .any(|r| r.url.path() == "/api/v2/app/setPreferences")
    );
    let added = form(
        requests
            .iter()
            .find(|r| r.url.path() == "/api/v2/torrents/add")
            .expect("add request"),
    );
    assert!(!added.contains_key("ratioLimit"));
    assert_eq!(added["stopped"], "false");
}

#[tokio::test]
async fn preference_failure_prevents_adding_torrents() {
    let server = server().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/app/setPreferences"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&server)
        .await;
    let opts = options(
        &server.uri(),
        &["--qbt-ratio-limit", "5", "--qbt-remove-at-ratio-limit"],
    )
    .expect("valid options");
    assert!(
        QbtClient::from(&opts)
            .add_torrents(magnets())
            .await
            .is_err()
    );
    let requests = server
        .received_requests()
        .await
        .expect("requests available");
    assert!(
        !requests
            .iter()
            .any(|r| r.url.path() == "/api/v2/torrents/add")
    );
}

#[test]
fn removal_requires_a_ratio_limit() {
    let error = options("http://localhost/", &["--qbt-remove-at-ratio-limit"])
        .expect_err("removal needs an explicit ratio");
    assert_eq!(
        error.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}
