use serde::{Serialize, Deserialize};
use duration_string::DurationString;
use std::time::SystemTime;
use std::process::Command;
use std::rc::Rc;
use std::path::Path;
use serde_json::Value;
use std::time::Duration;
use std::fs::File;
use std::io::Write;
use std::fs;
use log::{info,error};
use std::collections::HashMap;
use crate::app_config::AppConfig;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CacheEntry {
    pub description: String,
    pub pname: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cache {
    nixpkgs: HashMap<String, CacheEntry>,
}

impl Cache {
    pub fn package_iter<'a>(&'a self) -> std::collections::hash_map::Iter::<'a, String, CacheEntry> {
        self.nixpkgs.iter()
    }
}

fn read_cache(cache_file_path: &Path) -> Result<HashMap<String, CacheEntry>, String> {
    let nixpkgs_cache = match fs::read_to_string(cache_file_path) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("error reading nixpkg cache: {}", err);
            return Err("Couldn't read cache".to_string());
        }
    };

    let nixpkgs = serde_json::from_str(&nixpkgs_cache).unwrap();
    Ok(nixpkgs)
}

//TODO: implement
fn update_cache(cache_path: &Path) -> Result<HashMap<String, CacheEntry>, String> {
    let nixpkgs_json = match get_nixpkgs_json() {
        Ok(x) => x,
        Err(e) => {
            return Err(format!("failed to run nix search: {}", e));
        }
    };
    info!("saving cache to file: {:?}", cache_path);
    // nixpkgs_json
    let mut file = match File::create(&cache_path) {
        Ok(file) => file,
        Err(e) => {
            return Err(format!("failed to write to cache file: {}", e));
        }
    };
    match file.write(serde_json::to_string(&nixpkgs_json).unwrap().as_bytes()) {
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

fn get_nixpkgs_json() -> Result<HashMap<String, CacheEntry>, String> {
    let search_result = Command::new("nix").arg("search").arg("nixpkgs").arg("--json").arg("^").output();
    let search_output = match search_result {
        Ok(x) => String::from_utf8(x.stdout).unwrap(),
        Err(e) => {
            return Err(format!("Failed to run nix search command: {:?}", e));
        }
    };
    // info!("search result: {}", &search_output);
    info!("completed nix search command");
    let nixpkgs = serde_json::from_str(&search_output).unwrap();
    Ok(nixpkgs)
}

// pub fn get_cache(config: &AppConfig) -> Result<HashMap<String, CacheEntry>, String> {
pub fn get_cache(config: &AppConfig) -> Result<Cache, String> {
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
            update_cache(cache_path)
        } else {
            read_cache(cache_path)
        }
        // If the cache exists and is up to date, read and return it
    } else {
        // If it doesn't exist, create it and return it
        info!("cache does not exist");
        update_cache(cache_path)
    };
    // If we couldn't read it (but it existed) or we couldn't create it, error.
    match nixpkgs {
        Ok(nixpkgs) => { Ok(Cache {nixpkgs}) },
        Err(e) => { Err(format!("Could not read cache: '{}'", e)) }
    }
}
