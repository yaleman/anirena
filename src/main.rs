use clap::Parser;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts = anirena::cli::CLiOpts::parse();

    let mut client = anirena::AnirenaClient::new(opts.api_key)?;

    match opts.command {
        anirena::cli::Commands::Search { search_term, pages } => {
            println!("Searching for: {}", search_term.join(" "));
            client.search(search_term, None, pages).await?;
        }
    }
    Ok(())
}
