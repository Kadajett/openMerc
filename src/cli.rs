use clap::Parser;

/// Command line arguments for OpenMerc
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Run in headless mode with the given prompt
    #[clap(long, value_name = "PROMPT")]
    pub headless: Option<String>,

    /// Initialize a new configuration file
    #[clap(long)]
    pub init: bool,

    /// Resume a named session
    #[clap(long)]
    pub session: Option<String>,
}
