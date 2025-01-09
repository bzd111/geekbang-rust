use anyhow::Result;
use clap::Parser;
use rgrep::GrepConfig;

fn main() -> Result<()> {
    let config = GrepConfig::try_parse()?;
    config.match_with_default_strategy()?;
    Ok(())
}

// cargo run --quiet -- "Re[^\\s]+" "src/*.rs"
