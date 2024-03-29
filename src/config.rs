use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use std::time::Duration;
// use duration_string::DurationString;
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(version, about, long_about = None)]
pub struct Config {
    #[arg(short, long)]
    pub hm_config_file: String,
    #[arg(short, long)]
    pub cache_file_path: String,
    #[arg(short, long)]
    pub max_cache_age: String,
    #[arg(short, long)]
    pub num_print: usize,
}

