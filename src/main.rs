use clap::{Parser, Subcommand};
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};
use rnix::{self, SyntaxKind, SyntaxNode};
use log::{error,info};
use env_logger;
use colored::*;
use dialoguer::FuzzySelect;
use regex::Regex;
use std::cmp::min;
use std::process::Command;

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
    #[arg(long)]
    dry_run: bool,
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


fn pretty_format_result(result: &SearchResult) -> String {
    format!("{} \t [{}] \t {}", result.full_key, result.version, result.description)
}

fn pretty_print_result(result: &SearchResult) {
    println!("{}", pretty_format_result(result));
}

fn get_search_result_choice(results: &[SearchResult], max_length: usize) -> Option<usize> {
    let pretty_results: Vec<String> = results.iter().map(|x| pretty_format_result(x)).collect();
    FuzzySelect::new()
        .with_prompt("Choose package to install:")
        .items(&pretty_results)
        .max_length(max_length)
        .interact_opt()
        .unwrap()
}

fn run_hm_update() {
    info!("running home-manager switch");
    let update_command = Command::new("home-manager").arg("switch").output();
    match update_command {
	Ok(x) => {
	    info!("home-manager switch output: ");
	    info!("{:?}", x);
	},
	Err(e) => {
	    error!("home-manager switch error! output: ");
	    error!("{:?}", e);
	}
    }
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
            let change_made = nix_config.add_packages(&packages, &cache, cli_args.dry_run);
	    if change_made && app_config.hm_switch_afterwards {
		run_hm_update();
	    } else {
		info!("config not changed, not running home-manager switch");
	    }
        },
        Commands::Search { strings } => {
            info!("running search");
            let matching_results = search::search_cache(&strings, &app_config);
	    for result in &matching_results[0..min(matching_results.len(),app_config.num_search_results)] {
		pretty_print_result(result);
	    }
        }
    }
}
