use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};
use rnix::{self, SyntaxKind, SyntaxNode};
use log::{error,info};
use env_logger;
use colored::*;
use dialoguer::FuzzySelect;
use regex::Regex;
use std::cmp::min;
use std::process::Command;
use indicatif::ProgressBar;
use std::time::Duration;
use std::env::var;

mod search;
mod app_config;
mod nix_config;
mod cache;
use crate::nix_config::get_nix_config;
use crate::app_config::AppConfig;
use crate::search::SearchResult;
use crate::cache::get_cache;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(name = "fritz")]
#[command(about = "Manage packages in home-manager", long_about = None)]
struct Cli {
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    config: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, Serialize, Deserialize)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Add {
        packages: Vec<String>,
    },
    #[command(arg_required_else_help = true)]
    Rm {
        packages: Vec<String>,
    },
    #[command(arg_required_else_help = true)]
    Search {
        strings: Vec<String>,
    },
    List
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

fn run_hm_update(progress_bar: &ProgressBar, app_config: &AppConfig) {
    info!("running home-manager switch");
    progress_bar.set_message("running home-manager switch");
    let update_command = Command::new(&app_config.switch_base_command).arg("switch").output();
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
    progress_bar.inc(1)
}

fn add_changes(app_config: &AppConfig, progress_bar: &ProgressBar) {
    info!("addting config changes");
    progress_bar.set_message("git add");
    let config_file = std::path::Path::new(&app_config.package_config_file);
    let config_dir = config_file.parent().unwrap();
    let update_command_output = Command::new("git").arg("add").arg(format!("{}", config_file.file_name().unwrap().to_str().unwrap())).current_dir(config_dir).output().expect("failed to run git");
    let update_command_status = update_command_output.status;
    if update_command_status.success() {
	info!("git add output: ");
	info!("{:?}", &update_command_output.stdout);
    } else {
	error!("git add error! ");
	error!("{:?}", &update_command_output.stderr);
    }
    progress_bar.inc(1);
}


fn commit_changes(app_config: &AppConfig, progress_bar: &ProgressBar) {
    add_changes(app_config, progress_bar);
    progress_bar.set_message("git commit");
    info!("committing config changes");
    let config_file = std::path::Path::new(&app_config.package_config_file);
    let config_dir = config_file.parent().unwrap();
    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    let args: String = std::env::args().collect::<Vec<String>>().join(" ");
    let commit_msg = format!("fritz package update, command: {}", args);
    let update_command_output = Command::new("git").arg("commit").arg("-m").arg(commit_msg).current_dir(config_dir).output().expect("failed to run git");
    let update_command_status = update_command_output.status;
    bar.finish();
    if update_command_status.success() {
	info!("git commit output: ");
	info!("{:?}", &update_command_output.stdout);
    } else {
	error!("git commit error! ");
	error!("{:?}", &update_command_output.stdout);
	error!("{:?}", &update_command_output.stderr);
    }
    progress_bar.inc(1);
    match app_config.push_change {
	true => { push_changes(&app_config, &progress_bar); },
	false => { info!("pushing config changes is disabled") }
    }
}

fn push_changes(app_config: &AppConfig, progress_bar: &ProgressBar) {
    info!("pushing config changes");
    progress_bar.set_message("git push");
    let config_file = std::path::Path::new(&app_config.package_config_file);
    let config_dir = config_file.parent().unwrap();
    let update_command_output = Command::new("git").arg("push").current_dir(config_dir).output().expect("failed to run git");
    let update_command_status = update_command_output.status;
    if update_command_status.success() {
	info!("git push output: ");
	info!("{:?}", &update_command_output.stdout);
    } else {
	error!("git push error! ");
	error!("{:?}", &update_command_output.stderr);
    }
    progress_bar.inc(1);
}

fn get_default_config_file() -> String {
    let config_home = var("XDG_CONFIG_HOME").or_else(|_| var("HOME").map(|home| format!("{}/.config", home))).unwrap();
    format!("{}/fritz/config.toml", config_home)
}

fn remove_packages(packages: &Vec<String>, app_config: &AppConfig, cli_args: &Cli, progress_bar: &ProgressBar) {
    progress_bar.set_message("removing packages from config file");
    let nix_config = get_nix_config(&app_config);
    let change_made = nix_config.remove_packages(&packages, cli_args.dry_run);
    progress_bar.inc(1);
    if change_made && !cli_args.dry_run {
	match app_config.hm_switch {
	    true => { run_hm_update(progress_bar, app_config); },
	    false => { info!("home-manager switch is disabled") }
	}
	match app_config.commit_change {
	    true => {
		commit_changes(&app_config, progress_bar);
	    },
	    false => { info!("committing config changes is disabled") }
	}
    } else {
	info!("config was not changed");
	progress_bar.inc(app_config.hm_switch as u64
			 + 2*app_config.commit_change as u64
			 + app_config.push_change as u64);
    }
}

fn add_packages(packages: &Vec<String>, app_config: &AppConfig, cli_args: &Cli, progress_bar: &ProgressBar) {
    progress_bar.set_message("adding packages to config file");
    let nix_config = get_nix_config(&app_config);
    let cache = match get_cache(&app_config) {
	Ok(x) => x,
	Err(e) => {
	    error!("failed to read cache: {}", e);
	    return
	}
    };
    progress_bar.inc(1);
    let change_made = nix_config.add_packages(&packages, &cache, cli_args.dry_run);
    if change_made && !cli_args.dry_run {
	match app_config.hm_switch {
	    true => { run_hm_update(progress_bar, app_config); },
	    false => { info!("home-manager switch is disabled") }
	}
	match app_config.commit_change {
	    true => {
		commit_changes(&app_config, progress_bar);
	    },
	    false => { info!("committing config changes is disabled") }
	}
    } else {
	info!("config was not changed");
	progress_bar.inc(app_config.hm_switch as u64
			 + 2*app_config.commit_change as u64
			 + app_config.push_change as u64);
    }
}

fn list_packages(app_config: &AppConfig) {
    let nix_config = get_nix_config(&app_config);
    match nix_config.list_current_packages() {
	Some(found_packages) => {
	    for pkg in found_packages {
	    println!("{}", pkg);
	    }
	}
	None => {
	    error!("no packages found.");
	}
    }
}

fn get_progress_bar(app_config: &AppConfig, cli_args: &Cli) -> ProgressBar {
    let mut num_steps = 1
	+ app_config.hm_switch as u64
	+ 2*app_config.commit_change as u64
	+ app_config.push_change as u64;
    let progress_bar = ProgressBar::new(num_steps).with_style(
	indicatif::ProgressStyle::with_template("[{elapsed_precise}] {bar:40} {pos:>7}/{len:7} {wide_msg}").unwrap());
    progress_bar
}

fn main() {
    env_logger::init();
    let cli_args = Cli::parse();
    let config_file = match &cli_args.config {
	Some(x) => x.clone(),
	None => { get_default_config_file() }
    };
    info!("using config file: {}", config_file);
    let app_config: AppConfig = Figment::new()
        .merge(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(config_file))
        .merge(Env::prefixed("FRITZ_"))
        .extract().unwrap();

    let progress_bar = get_progress_bar(&app_config, &cli_args);
    match cli_args.command {
        Commands::Add {ref packages} => { add_packages(packages, &app_config, &cli_args, &progress_bar) },
        Commands::Rm {ref packages} => { remove_packages(packages, &app_config, &cli_args, &progress_bar) },
        Commands::Search { strings } => {
            info!("running search");
            let matching_results = search::search_cache(&strings, &app_config);
	    for result in &matching_results[0..min(matching_results.len(),app_config.num_search_results)] {
		pretty_print_result(result);
	    }
        },
	Commands::List => {
	    info!("listing fritz-managed packages");
	    list_packages(&app_config);
	}
    }
}
