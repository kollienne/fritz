use rnix::{self, SyntaxKind, SyntaxNode};
use std::fs;
use nix_editor;
use log::info;
use itertools::Itertools;

use crate::AppConfig;
use crate::cache::get_cache;

pub fn get_nix_config(app_config: &AppConfig) -> NixConfig {
    info!("reading config file: {}", app_config.hm_config_file);
    let cf_path = &app_config.hm_config_file;
    let current_packages = match get_current_packages(&app_config.hm_config_file) {
        Some(cfg) => {
            cfg
        },
        None => {
            eprintln!("Unable to read config file {}", app_config.hm_config_file);
            std::process::exit(1);
        },
    };
    NixConfig {
        app_config: app_config.clone(),
        current_packages
    }
}

pub struct NixConfig {
    app_config: AppConfig,
    current_packages: SyntaxNode,
}

impl NixConfig {
    fn initialise() {
    }

    fn get_full_package_name(self, short_name: String) {
        let cache = get_cache(&self.app_config);
    }

    pub fn add_packages(self, packages: &Vec<String>) {
        println!("Trying to add package(s) {:?}", packages);
        println!("current packages: {}", &self.current_packages);
        match Self::config_subset_not_present(&packages, &self.current_packages) {
            Some(package_subset) => {
                info!("adding subset: {:?}", &package_subset);
                for package in &package_subset {
                    
                }
                let new_str = match addtoarr_aux(&self.current_packages, package_subset) {
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
        }
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

pub fn get_current_packages(config_file: &String) -> Option<SyntaxNode> {
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
