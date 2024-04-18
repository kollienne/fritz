use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use std::time::Duration;
use std::env::var;
use std::path::Path;
// use duration_string::DurationString;
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};

#[derive(Parser, Debug, Serialize, Deserialize, Clone)]
#[command(version, about, long_about = None)]
pub struct AppConfig {
    #[arg(short, long)]
    pub package_config_file: String,
    #[arg(short, long)]
    pub cache_file_path: String,
    #[arg(short, long)]
    pub max_cache_age: String,
    #[arg(short, long)]
    pub num_print: usize,
    #[arg(short, long)]
    pub num_search_results: usize,
    #[arg(short, long)]
    pub commit_change: bool,
    #[arg(short, long)]
    pub push_change: bool,
    #[arg(short, long)]
    pub hm_switch: bool,
}

impl Default for AppConfig {
    fn default() -> AppConfig {
	let config_home = var("XDG_CONFIG_HOME").or_else(|_| var("HOME").map(|home| format!("{}/.config", home))).unwrap();
	let package_config_file = format!("{}/home-manager/fritz-packages.nix", config_home);
	let cache_file_path = format!("{}/fritz/nixpkgs_cache.msgpack", config_home);
	
        AppConfig {
	    package_config_file,
            cache_file_path,
            max_cache_age: "12h".to_string(),
            num_print: 10,
            num_search_results: 10,
	    commit_change: true,
	    push_change: true,
	    hm_switch: true
        }
    }
}
