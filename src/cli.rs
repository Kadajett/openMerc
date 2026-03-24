use clap::Parser;

/// Command line arguments for OpenMerc
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Run in headless mode with the given prompt
    #[clap(long, requires = "prompt")]
    pub headless: bool,

    /// Prompt to use when headless mode is enabled
    #[clap(long, requires = "headless")]
    pub prompt: Option<String>,

    /// Initialize a new configuration file
    #[clap(long)]
    pub init: bool,

    /// Resume a named session
    #[clap(long)]
    pub session: Option<String>,
}
