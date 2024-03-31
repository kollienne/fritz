use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use serde_json::Value;
use std::fs::File;
use std::io::Write;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::SystemTime;
use std::time::Duration;
use std::process::Command;
use crate::app_config::AppConfig;
use duration_string::DurationString;
use log::{info,error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub full_key: String,
    pub description: String,
    pub pname: String,
    pub version: String,
    pub desc_score: f32,
    pub key_score: f32,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct SearchResultInternal {
    description: String,
    pname: String,
    version: String,
}

fn score_result(key: &String, result: &SearchResultInternal, search_strings: &Vec<String>) -> Option<SearchResult> {
    let mut desc_term_freq = 0;
    let mut key_term_freq = 0;
    for string in search_strings {
        if result.description.to_lowercase().contains(string) {
            desc_term_freq += string.len();
        }
        if result.pname.to_lowercase().contains(string) {
            key_term_freq += string.len();
        }
    }
    if desc_term_freq + key_term_freq > 0 {
        let desc_score = if desc_term_freq > 0 && result.description.len() > 0 {
            (desc_term_freq as f32) / (result.description.len() as f32)
        } else {
            0.0
        };
        let key_score = if key_term_freq > 0 && result.pname.len() > 0 {
            (key_term_freq as f32) / (result.pname.len() as f32)
        } else {
            0.0
        };
        Some(SearchResult{
            full_key: key.clone(),
            description: result.description.clone(),
            pname: result.pname.clone(),
            version: result.version.clone(),
            desc_score,
            key_score})
    } else {
        None
    }
}

pub fn search_cache(strings: &Vec<String>, config: &AppConfig) -> Vec<SearchResult> {
    let max_cache_duration = config.max_cache_age.parse::<DurationString>().unwrap().into();
    let cache = get_cache(&config.cache_file_path, max_cache_duration).unwrap();
    info!("cache: {:?}", cache["legacyPackages.x86_64-linux.AMB-plugins"]);
    let mut matching_results: Vec<SearchResult> = cache.iter().filter_map(|(key, result)| score_result(key, result, &strings)).collect();
    matching_results.sort_by(|a, b| b.desc_score.partial_cmp(&a.desc_score).unwrap());
    matching_results.sort_by(|a, b| b.key_score.partial_cmp(&a.key_score).unwrap());
    info!("{} matching results", matching_results.len());
    // info!("top result: {:?}", matching_results[0]);
    matching_results
}

fn read_cache(cache_file_path: &Path) -> Result<HashMap<String, SearchResultInternal>, String> {
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
fn update_cache(cache_path: &Path) -> Result<HashMap<String, SearchResultInternal>, String> {
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

fn get_nixpkgs_json() -> Result<HashMap<String, SearchResultInternal>, String> {
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

fn get_cache(cache_path_str: &String, max_cache_age: Duration) -> Result<HashMap<String, SearchResultInternal>, String> {
    info!("attempting to read cache: {}", cache_path_str);
    let cache_path = Path::new(cache_path_str);
    let cache_exists = match cache_path.try_exists() {
        Ok(x) => x,
        Err(err) => {
            eprintln!("error checking whether cache exists: {}", err);
            return Err(format!("Failed to check path {}", cache_path_str));
        }
    };
    let cache = if cache_exists {
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
    cache
}
