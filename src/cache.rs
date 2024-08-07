use serde::{Serialize, Deserialize};
use rmp_serde;
use duration_string::DurationString;
use std::{time::SystemTime, process::exit};
use std::process::Command;
use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::fs;
use log::{info,error};
use std::collections::HashMap;
use crate::app_config::AppConfig;
use indicatif::ProgressBar;
use platform_info::{PlatformInfo,PlatformInfoAPI,UNameAPI};

const PB_NUM_STEPS:     u64 = 3;
const PB_START:         u64 = 1;
const PB_CACHE_FETCHED: u64 = 2;
const PB_CACHE_PARSED:  u64 = 3;

fn get_platform_string() -> String {
    let info = PlatformInfo::new().expect("Unable to determine platform");
    println!("{:?}", info);
    let current_platform = info.sysname().to_string_lossy().to_lowercase();
    let current_arch  = info.machine().to_string_lossy().to_lowercase();
    let platform_string = format!("{}-{}", current_arch, current_platform);
    match &platform_string[..] {
        "x86_64-linux-gnu" => "legacyPackages.x86_64-linux".to_string(),
        "arm64-darwin" => "legacyPackages.aarch64-darwin".to_string(),
        _ => {
            error!("unknown platform: {}", current_platform);
            exit(1);
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub description: String,
    pub pname: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cache {
    pub nixpkgs: HashMap<String, CacheEntry>,
}

impl Cache {
    pub fn package_iter<'a>(&'a self) -> std::collections::hash_map::Iter::<'a, String, CacheEntry> {
        self.nixpkgs.iter()
    }
}

fn read_cache(cache_file_path: &Path) -> Result<Cache, String> {
    let nixpkgs_cache = match fs::read(cache_file_path) {
        Ok(x) => {
	    let nixpkgs: Cache = rmp_serde::from_slice(&x).unwrap();
	    Ok(nixpkgs)
	},
        Err(err) => {
            eprintln!("error reading nixpkg cache: {}", err);
            return Err("Couldn't read cache".to_string());
        }
    };
    nixpkgs_cache
}

fn update_cache(cache_path: &Path, progress_bar: Option<&ProgressBar>) -> Result<Cache, String> {
    let nixpkgs_json = match get_nixpkgs_json(progress_bar) {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("failed to run nix search: {}", e));
        }
    };
    info!("saving cache to file: {:?}", cache_path);
    // nixpkgs_json
    let parent_dir = Path::new(cache_path).parent().unwrap();
    if !parent_dir.exists() {
	info!("cache directory '{}' does not exist, creating it.", parent_dir.to_str().unwrap());
    }
    match fs::create_dir_all(parent_dir) {
	    Ok(_) => (),
	    Err(x) => {
		error!("{:?}", x);
	    }
    };
    let mut file = match File::create(&cache_path) {
        Ok(file) => file,
        Err(e) => {
            return Err(format!("failed to write to cache file: {}", e));
        }
    };
    match file.write(&rmp_serde::to_vec(&nixpkgs_json).unwrap()) {
        Ok(_) => {
            info!("successfully wrote cache file");
        },
        Err(e) => {
            error!("failed to write to cache file: {}", e);
            return Err("failed to write to cache file".to_string());
        }
    }
    Ok(nixpkgs_json)
}

fn get_nixpkgs_json(progress_bar: Option<&ProgressBar>) -> Result<Cache, String> {
    if progress_bar.is_some() { progress_bar.unwrap().set_position(PB_START); }
    if progress_bar.is_some() { progress_bar.unwrap().set_message("fetching nixpkgs index"); }
    let search_result = Command::new("nix").arg("search").arg("nixpkgs").arg("--json").arg("^").output();
    let search_output = match search_result {
        Ok(x) => String::from_utf8(x.stdout).unwrap(),
        Err(e) => {
            return Err(format!("Failed to run nix search command: {:?}", e));
        }
    };
    if progress_bar.is_some() { progress_bar.unwrap().set_position(PB_CACHE_FETCHED); }
    if progress_bar.is_some() { progress_bar.unwrap().set_message("parsing nixpkgs"); }
    info!("completed nix search command");

    let search_output = search_output.replace(&get_platform_string(), "pkgs");
    let nixpkgs = serde_json::from_str(&search_output).unwrap();
    let nixpkgs = Cache { nixpkgs };
    if progress_bar.is_some() { progress_bar.unwrap().set_position(PB_CACHE_PARSED); }
    Ok(nixpkgs)
}

pub fn get_cache(config: &AppConfig) -> Result<Cache, String> {
    let progress_bar = ProgressBar::new(PB_NUM_STEPS).with_style(
	indicatif::ProgressStyle::with_template("[{elapsed_precise}] {bar:40} {pos:>7}/{len:7} {wide_msg}").unwrap());
    let max_cache_age = config.max_cache_age.parse::<DurationString>().unwrap().into();
    let cache_path_str = &config.cache_file_path;
    info!("attempting to read cache: {}", &cache_path_str);
    let cache_path = Path::new(&cache_path_str);
    let cache_exists = match cache_path.try_exists() {
        Ok(x) => x,
        Err(err) => {
            eprintln!("error checking whether cache exists: {}", err);
            return Err(format!("Failed to check path {}", cache_path_str));
        }
    };
    let nixpkgs = if cache_exists {
        info!("cache exists");
        // If it's old, update it and return it
        let cache_metadata = cache_path.metadata().expect("Failed to read metadata from cache");
        let last_mod_time = cache_metadata.modified().expect("failed to read cache last modified time");
        let cache_age = SystemTime::now().duration_since(last_mod_time).unwrap();
        if cache_age > max_cache_age {
            info!("cache is {:.1} minutes old, updating cache", cache_age.as_secs_f32() / 60.0);
            update_cache(cache_path, Some(&progress_bar))
        } else {
            read_cache(cache_path)
        }
        // If the cache exists and is up to date, read and return it
    } else {
        // If it doesn't exist, create it and return it
        info!("cache does not exist");
        update_cache(cache_path, Some(&progress_bar))
    };
    // If we couldn't read it (but it existed) or we couldn't create it, error.
    match nixpkgs {
        Ok(nixpkgs) => { Ok(nixpkgs) },
        Err(e) => { Err(format!("Could not read cache: '{}'", e)) }
    }
}
