use clap::Parser;

#[derive(Parser, Debug)]
pub struct CLiOpts {
    #[clap(long, env = "ANIRENA_API_KEY", hide_env_values = true)]
    pub api_key: String,

    #[clap(long, env = "QBT_BASE_URL")]
    pub qbt_base_url: String,

    #[clap(long, env = "QBT_USERNAME")]
    pub qbt_username: String,

    #[clap(long, env = "QBT_PASSWORD", hide_env_values = true)]
    pub qbt_password: String,

    #[clap(long, env = "QBT_CATEGORY")]
    pub qbt_category: Option<String>,

    #[clap(long, env = "QBT_AUTO_START")]
    pub qbt_auto_start: bool,

    /// Share ratio limit for torrents added by this command (for example, 5).
    #[clap(long, env = "QBT_RATIO_LIMIT", value_parser = parse_ratio_limit)]
    pub qbt_ratio_limit: Option<f64>,

    /// Remove torrents at their share limit, keeping downloaded files.
    /// Changes qBittorrent's GLOBAL action, including for existing torrents.
    #[clap(long, env = "QBT_REMOVE_AT_RATIO_LIMIT", requires = "qbt_ratio_limit")]
    pub qbt_remove_at_ratio_limit: bool,

    #[clap(long)]
    pub pages: Option<u32>,

    #[clap(long)]
    pub add: bool,

    pub search_term: Vec<String>,
}

fn parse_ratio_limit(value: &str) -> Result<f64, String> {
    let ratio: f64 = value.parse().map_err(|_| "ratio must be a number")?;
    if ratio.is_finite() && ratio >= 0.0 {
        Ok(ratio)
    } else {
        Err("ratio must be finite and non-negative".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ratio_limit_rejects_invalid_values() {
        for value in ["-1", "NaN", "inf", "-inf", "abc"] {
            assert!(parse_ratio_limit(value).is_err(), "accepted {value}");
        }
        assert_eq!(parse_ratio_limit("5").expect("valid ratio"), 5.0);
        assert_eq!(parse_ratio_limit("0").expect("valid ratio"), 0.0);
    }
}
