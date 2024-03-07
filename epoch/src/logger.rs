use log::*;
use simplelog::{ColorChoice, Config, TermLogger, TerminalMode};

pub fn init_logger() -> anyhow::Result<()> {
    Ok(TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )?)
}
