mod config;
mod schema;

pub use config::*;
pub use schema::*;

use clap::ValueEnum;

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Yaml,
    Raw,
    Json,
    Properties,
}
