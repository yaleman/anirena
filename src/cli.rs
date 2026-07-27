use clap::{Parser, Subcommand};
use serde::Deserialize;

#[derive(Parser, Debug)]
pub struct CLiOpts {
    // #[clap(env = "ANIRENA_USERNAME")]
    // pub username: String,
    // #[clap(env = "ANIRENA_PASSWORD")]
    // pub password: String,
    // #[clap(env = "ANIRENA_TOTP_SECRET")]
    // totp_secret: String,
    #[clap(env = "ANIRENA_API_KEY")]
    pub api_key: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone, Deserialize)]
#[clap(rename_all = "kebab-case")]
pub enum Commands {
    Search {
        search_term: Vec<String>,
        #[clap(long)]
        pages: Option<u32>,
    },
}
