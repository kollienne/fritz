use clap::{Parser, Subcommand};
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};
use rnix::{self, SyntaxKind, SyntaxNode};
use nix_editor;
use std::fs;
use itertools::Itertools;
use log::info;
use env_logger;
use colored::*;

mod search;
mod config;
use crate::config::Config;
use crate::search::SearchResult;

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

impl Default for Config {
    fn default() -> Config {
        Config {
            hm_config_file: "./home.nix".into(),
            cache_file_path: "./nixpkgs_cache.json".into(),
            max_cache_age: "12h".to_string(),
            num_print: 10
        }
    }
}

fn get_current_packages(config_file: &String) -> Option<SyntaxNode> {
    // let content = fs::read_to_string(config_file)?; 
    let content = match fs::read_to_string(config_file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("error reading file: {}", err);
            return None;
        }
    };
    let parsed = rnix::Root::parse(&content);
    let ast = rnix::Root::parse(config_file);
    let sn = &ast.syntax();
    let expr = match ast.tree().expr() {
        Some(expr) => expr,
        None => {
            eprintln!("error reading file");
            return None;
        }
    };

    let configbase = match nix_editor::parse::getcfgbase(&parsed.syntax()) {
        Some(x) => x,
        None => {
            eprintln!("could not parse {}", config_file);
            return None;
        }
    };
    let packages = match nix_editor::parse::findattr(&configbase, &"home.packages") {
        Some(attr) => attr,
        None => {
            return None;
        }
    };
    Some(packages)
}

// borowwed from github.com/snowfallorg/nix-editor
fn addtoarr_aux(node: &SyntaxNode, items: Vec<String>) -> Option<SyntaxNode> {
    for child in node.children() {
        if child.kind() == rnix::SyntaxKind::NODE_WITH {
            return addtoarr_aux(&child, items);
        }
        if child.kind() == SyntaxKind::NODE_LIST {
            let mut green = child.green().into_owned();

            for elem in items {
                let mut i = 0;
                for c in green.children() {
                    if c.to_string() == "]" {
                        if green.children().collect::<Vec<_>>()[i - 1]
                            .as_token()
                            .unwrap()
                            .to_string()
                            .contains('\n')
                        {
                            i -= 1;
                        }
                        green = green.insert_child(
                            i,
                            rnix::NodeOrToken::Node(
                                rnix::Root::parse(&format!("\n{}{}", " ".repeat(4), elem))
                                    .syntax()
                                    .green()
                                    .into_owned(),
                            ),
                        );
                        break;
                    }
                    i += 1;
                }
            }

            let index = match node.green().children().position(|x| match x.into_node() {
                Some(x) => x.to_owned() == child.green().into_owned(),
                None => false,
            }) {
                Some(x) => x,
                None => return None,
            };

            let replace = node
                .green()
                .replace_child(index, rnix::NodeOrToken::Node(green));
            let out = node.replace_with(replace);
            let output = rnix::Root::parse(&out.to_string()).syntax();
            return Some(output);
        }
    }
    None
}

fn config_subset_not_present(packages: &Vec<String>, config: &SyntaxNode) -> Option<Vec<String>> {
    let config_str = config.to_string();

    let subset = packages.iter().unique().filter(|x| !config_str.contains(*x)).cloned().collect::<Vec<String>>();
    info!("returning subset: {:?}", subset);
    if subset.len() > 0 {
        Some(subset)
    } else {
        None
    }
}

fn pretty_print_result(result: &SearchResult, search_strings: &Vec<String>) {
    println!("{:?}", result);
}

fn main() {
    env_logger::init();
    let config:Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file("Config.toml"))
        .merge(Env::prefixed("APP_"))
        .extract().unwrap();

    info!("reading config file: {}", config.hm_config_file);
    let cf_path = &config.hm_config_file;
    let current_packages = match get_current_packages(&config.hm_config_file) {
        Some(cfg) => {
            cfg
        },
        None => {
            eprintln!("Unable to read config file {}", config.hm_config_file);
            std::process::exit(1);
        },
    };

    let cli_args = Cli::parse();
    match cli_args.command {
        Commands::Add {packages} => {
            println!("Trying to add package(s) {:?}", packages);
            println!("current packages: {}", current_packages);
            match config_subset_not_present(&packages, &current_packages) {
                Some(package_subset) => {
                    info!("adding subset: {:?}", package_subset);
                    let new_str = match addtoarr_aux(&current_packages, package_subset) {
                        Some(new_str) => new_str,
                        None => {
                            eprintln!("error adding package");
                            std::process::exit(1);
                        }
                    };
                    info!("{}", new_str);
                },
                None => {
                    info!("All packages already present")
                }
            };
        },
        Commands::Search { strings } => {
            info!("running search");
            let matching_results = search::search_cache(&strings, &config);
            for result in matching_results.iter().take(config.num_print) {
                // println!("{:?}", result);
                pretty_print_result(result, &strings)
            }
        }
    }
}
