use clap::Parser;

#[derive(Parser, Debug)]
pub struct CLiOpts {
    #[clap(long, env = "ANIRENA_API_KEY")]
    pub api_key: String,

    #[clap(long, env = "QBT_BASE_URL")]
    pub qbt_base_url: String,

    #[clap(long, env = "QBT_USERNAME")]
    pub qbt_username: String,

    #[clap(long, env = "QBT_PASSWORD")]
    pub qbt_password: String,

    #[clap(long, env = "QBT_CATEGORY")]
    pub qbt_category: Option<String>,

    #[clap(long, env = "QBT_AUTO_START")]
    pub qbt_auto_start: bool,

    #[clap(long)]
    pub pages: Option<u32>,

    #[clap(long)]
    pub add: bool,

    pub search_term: Vec<String>,
}
