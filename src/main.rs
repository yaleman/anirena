use std::{
    io::{self, IsTerminal},
    str::FromStr,
};

use clap::Parser;
use qbit_rs::model::{AddTorrentArg, TorrentSource};
use reqwest::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = anirena::cli::CLiOpts::parse();

    let mut client = anirena::AnirenaClient::new(&opts.api_key)?;

    if opts.search_term.is_empty() {
        eprintln!("Error: No search term provided.");
        std::process::exit(1);
    }

    println!("Searching for: {}", opts.search_term.join(" "));
    let results = client.search(&opts.search_term, None, opts.pages).await?;
    if !opts.add {
        let hyperlinks_enabled = io::stdout().is_terminal();
        for torrent in results {
            let group_tag = torrent
                .group_name
                .as_deref()
                .map(|g| format!("[{g}] "))
                .unwrap_or("".to_string());
            println!(
                "{} {}, Size: {}, Seeders: {}, Leechers: {}",
                group_tag, torrent.title, torrent.size_fmt, torrent.seeders, torrent.leechers
            );
            println!(
                "{}",
                anirena::format_magnet_line(&torrent.magnet, hyperlinks_enabled)
            );
        }
    } else {
        let to_add = dialoguer::MultiSelect::new()
            .with_prompt("Select torrents to add")
            .items(
                results
                    .iter()
                    .map(|t| format!("{} - {}", t.title, t.size_fmt))
                    .collect::<Vec<_>>(),
            )
            .interact()
            .unwrap_or_default();
        if to_add.is_empty() {
            println!("No torrents selected. Exiting.");
            return Ok(());
        }

        println!("Adding {} torrents to qBittorrent...", to_add.len());
        let qbt = anirena::qbt::QbtClient::from(&opts).login().await?;
        let mut args = AddTorrentArg::default();
        if let Some(category) = opts.qbt_category {
            args.category = Some(category);
        }
        args.stopped = Some((!opts.qbt_auto_start).to_string());

        let magnet_urls = results
            .iter()
            .enumerate()
            .filter_map(|(index, torrent)| {
                if to_add.contains(&index) {
                    Some(
                        Url::from_str(&torrent.magnet.clone())
                            .expect("FFailed to parse magnet URL"),
                    )
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        args.source = TorrentSource::Urls {
            urls: magnet_urls.into(),
        };
        qbt.add_torrent(args).await?;
    }
    Ok(())
}
