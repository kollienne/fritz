use rnix::{self, SyntaxKind, SyntaxNode};
use std::fs::{File,read_to_string};
use std::io::{Write,stdin};
use std::collections::HashSet;
use nix_editor;
use log::{info,error};
use itertools::Itertools;

use crate::AppConfig;
use crate::cache::{Cache,get_cache};

pub fn get_nix_config(app_config: &AppConfig) -> NixConfig {
    info!("reading config file: {}", app_config.package_config_file);
    let cf_path = &app_config.package_config_file;
    let current_packages = match get_current_packages(&app_config.package_config_file) {
        Some(cfg) => {
            cfg
        },
        None => {
            eprintln!("Unable to read config file {}", app_config.package_config_file);
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

fn update_config_file(config_file_path: &String, new_str: &String) {
    let mut config_file = match File::create(config_file_path) {
        Ok(x) => x,
        Err(e) => {
            error!("Could not write config file {}: {}", config_file_path, e);
            return
        }
    };
    let _ = config_file.write_all(new_str.as_bytes());
}

fn config_contains_key(node: &SyntaxNode, item: &String) -> bool {
    for child in node.children() {
	if child.kind() == rnix::SyntaxKind::NODE_WITH {
	    return config_contains_key(&child, item);
	}
	if child.kind() == SyntaxKind::NODE_LIST {
	    for c in child.green().children() {
		if c.to_string().eq(item) {
		    return true
		}
	    }
	}
    }
    false
}

impl NixConfig {
    fn get_full_package_name(&self, short_name: &String, cache: &Cache) -> Option<String> {
        if cache.nixpkgs.contains_key(short_name) {
            // exact match
            Some(short_name.clone())
        } else {
            // try pkgs.{short_name}
            let test_str = format!("pkgs.{}", short_name);
            if cache.nixpkgs.contains_key(&test_str) {
                info!("found full package name for '{}': {}", short_name, test_str);
                Some(test_str)
            } else {
                error!("no full package name found for '{}'", short_name);
                None
            }
        }
    }

    fn get_full_package_name_from_config(&self, short_name: &String) -> Option<String> {
	if config_contains_key(&self.current_packages, short_name) {
	    info!("found full package name for '{}': {}", short_name, short_name);
            Some(short_name.clone())
	} else {
            let test_str = format!("pkgs.{}", short_name);
	    if config_contains_key(&self.current_packages, &test_str) {
                info!("found full package name for '{}': {}", short_name, &test_str);
		Some(test_str)
	    } else {
                error!("no full package name found for '{}'", short_name);
		None
	    }
	}
    }

    pub fn add_packages(&self, packages: &Vec<String>, cache: &Cache, dry_run: bool) -> bool {
        println!("Trying to add package(s) {:?}", packages);
        let full_package_set: Vec<String> = packages.iter().filter_map(
            |short_name| {
                self.get_full_package_name(short_name, cache)
            }
        ).collect();
        // Ask to continue if not everything was found
        //TODO: mention which packages were not found
        if full_package_set.len() != packages.len() {
            println!("Some packages were not found, continue? (Y/N): ");
            let mut buffer = String::new();
            stdin().read_line(&mut buffer).unwrap();
            if buffer.to_lowercase() != "y\n" {
                return false
            }
        }
        let change_made = match Self::config_subset_not_present(&full_package_set, &self.current_packages) {
            Some(package_subset) => {
                info!("adding subset: {:?}", &package_subset);
                let new_str = match addtoarr_aux(&self.current_packages, package_subset) {
                Some(new_str) => new_str,
                    None => {
                        eprintln!("error adding package");
                        std::process::exit(1);
                    }
                };
                info!("updating config file: {}", self.app_config.package_config_file);
                // replace config with new_str, then commit with git.
                if !dry_run {
                    update_config_file(&self.app_config.package_config_file, &new_str.to_string());
		    true
                } else {
                    info!("dry run, not actually updating file");
		    false
                }
            },
            None => {
                info!("All packages already present");
		false
            }
        };
	change_made
    }

    fn get_package_subset_in_config(&self, packages: &Vec<String>) -> (Vec<String>,Vec<String>) {
	let mut found_subset = vec![];
	let mut not_found_subset = vec![];
	for try_package in packages {
	    match self.get_full_package_name_from_config(try_package) {
		Some(x) => { found_subset.push(x.clone()) },
		None => { not_found_subset.push(try_package.clone()) }
	    };
	};
	(found_subset, not_found_subset)
    }

    pub fn remove_packages(&self, packages: &Vec<String>, dry_run: bool) -> bool {
        println!("Trying to remove package(s) {:?}", packages);
	let (full_package_set, not_found_subset) = self.get_package_subset_in_config(packages);
        // Ask to continue if not everything was found
        if full_package_set.len() != packages.len() {
	    println!("Packages not found: ");
	    for ps in not_found_subset {
		print!("{}, ", ps);
	    }
	    println!();
            println!("Some packages were not found, continue? (Y/N): ");
            let mut buffer = String::new();
            stdin().read_line(&mut buffer).unwrap();
            if buffer.to_lowercase() != "y\n" {
                return false
            }
        }
	let change_made = if packages.len() > 0 {
	    info!("removing subset: {:?}", &full_package_set);
	    let new_str = match rmarr_aux(&self.current_packages, &full_package_set) {
	    Some(new_str) => new_str,
		None => {
		    eprintln!("error removing package");
		    std::process::exit(1);
		}
	    };
	    info!("updating config file: {}", self.app_config.package_config_file);
	    // replace config with new_str, then commit with git.
	    if !dry_run {
		update_config_file(&self.app_config.package_config_file, &new_str.to_string());
		true
	    } else {
		info!("dry run, not actually updating file");
		false
	    }
	} else {
	    info!("All packages already present");
	    false
        };
	change_made
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

// borowwed from github.com/snowfallorg/nix-editor
fn rmarr_aux(node: &SyntaxNode, items: &Vec<String>) -> Option<SyntaxNode> {
    for child in node.children() {
        if child.kind() == rnix::SyntaxKind::NODE_WITH {
            return rmarr_aux(&child, items);
        }
        if child.kind() == SyntaxKind::NODE_LIST {
            let green = child.green().into_owned();
            let mut idx = vec![];
            for elem in green.children() {
                if elem.as_node().is_some() && items.contains(&elem.to_string()) {
                    let index = match green.children().position(|x| match x.into_node() {
                        Some(x) => {
                            if let Some(y) = elem.as_node() {
                                x.eq(y)
                            } else {
                                false
                            }
                        }
                        None => false,
                    }) {
                        Some(x) => x,
                        None => return None,
                    };
                    idx.push(index)
                }
            }
            let mut acc = 0;
            let mut replace = green;

            for i in idx {
                replace = replace.remove_child(i - acc);
                let mut v = vec![];
                for c in replace.children() {
                    v.push(c);
                }
                if let Some(x) = v.get(i - acc - 1).unwrap().as_token() {
                    if x.to_string().contains('\n') {
                        replace = replace.remove_child(i - acc - 1);
                        acc += 1;
                    }
                }
                acc += 1;
            }
            let out = child.replace_with(replace);

            let output = rnix::Root::parse(&out.to_string()).syntax();
            return Some(output);
        }
    }
    None
}


pub fn get_current_packages(config_file: &String) -> Option<SyntaxNode> {
    // let content = fs::read_to_string(config_file)?; 
    let content = match read_to_string(config_file) {
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
