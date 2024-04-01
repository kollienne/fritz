use clap::{Parser, Subcommand};
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};
use rnix::{self, SyntaxKind, SyntaxNode};
use log::{error,info};
use env_logger;
use colored::*;

mod search;
mod app_config;
mod nix_config;
mod cache;
use crate::nix_config::get_nix_config;
use crate::app_config::AppConfig;
use crate::search::SearchResult;
use crate::cache::get_cache;

#[derive(Parser, Debug)]
#[command(name = "add")]
#[command(about = "Add packages to home-manager config file", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Add {
        packages: Vec<String>,
    },
    Search {
        strings: Vec<String>,
    }
}

fn pretty_print_result(result: &SearchResult, search_strings: &Vec<String>) {
    println!("{:?}", result);
}

fn main() {
    env_logger::init();
    let app_config: AppConfig = Figment::new()
        .merge(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file("Config.toml"))
        .merge(Env::prefixed("APP_"))
        .extract().unwrap();


    let cli_args = Cli::parse();
    match cli_args.command {
        Commands::Add {packages} => {
            let nix_config = get_nix_config(&app_config);
            let cache = match get_cache(&app_config) {
                Ok(x) => x,
                Err(e) => {
                    error!("failed to read cache: {}", e);
                    return
                }
            };
            nix_config.add_packages(&packages, &cache);
        },
        Commands::Search { strings } => {
            info!("running search");
            let matching_results = search::search_cache(&strings, &app_config);
            for result in matching_results.iter().take(app_config.num_print) {
                // println!("{:?}", result);
                pretty_print_result(result, &strings)
            }
        }
    }
}
