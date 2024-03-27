use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};
use rnix::{self, SyntaxKind, SyntaxNode};
use nix_editor;
use std::fs;

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(version, about, long_about = None)]
struct Config {
    #[arg(short, long)]
    hm_config_file: String
}

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
        pkg: String,
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            hm_config_file: "./home.nix".into()
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

fn main() {
    let config:Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file("Config.toml"))
        .merge(Env::prefixed("APP_"))
        .extract().unwrap();

    println!("reading config file: {}", config.hm_config_file);
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

    println!("current packages: {}", current_packages);
    let current_packages_string = current_packages.to_string();

    let cli_args = Cli::parse();
    match cli_args.command {
        Commands::Add {pkg} => {
            println!("Adding package {}", pkg);
            let new_str = match addtoarr_aux(&current_packages, vec![pkg]) {
                Some(new_str) => new_str,
                None => {
                    eprintln!("error adding package");
                    std::process::exit(1);
                }
            };
            println!("{}", new_str);
        }
    }
}
