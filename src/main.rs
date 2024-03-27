use clap::{Parser, Subcommand};
use serde::{Serialize, Deserialize};
use figment::{Figment, providers::{Serialized, Toml, Env, Format}};
use rnix::{self, SyntaxKind, SyntaxNode};
use nix_editor;

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

fn get_current_packages(config_file: &String) -> Result<String, nix_editor::read::ReadError> {
    let ast = rnix::Root::parse(config_file);
    let sn = &ast.syntax();
    let kind = sn.kind();
    let configbase = match nix_editor::parse::getcfgbase(&ast.syntax()) {
        Some(x) => x,
        None => {
            eprintln!("could not parse {}", config_file);
            return Err(nix_editor::read::ReadError::ParseError)
        }
    };
    let current_packages = nix_editor::read::readvalue(&config_file, &"home.packages");
    current_packages
}

fn main() {
    let config:Config = Figment::new()
        .merge(Serialized::defaults(Config::default()))
        .merge(Toml::file("Config.toml"))
        .merge(Env::prefixed("APP_"))
        .extract().unwrap();

    println!("reading config file: {}", config.hm_config_file);
    let current_config = get_current_packages(&config.hm_config_file);

    match current_config {
        Err(e) => {
            eprintln!("Unable to read config file {}", config.hm_config_file);
            eprintln!("Error: {:#?}", e);
            std::process::exit(1);
        },
        Ok(cfg) => {
            
        }
    }

    let cli_args = Cli::parse();
    match cli_args.command {
        Commands::Add {pkg} => {
            println!("Adding package {}", pkg);
        }
    }
}
