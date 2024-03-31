use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use std::process::Command;
use duration_string::DurationString;
use log::{info,error};

use crate::app_config::AppConfig;
use crate::cache::{Cache,CacheEntry, get_cache};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchResult {
    pub full_key: String,
    pub description: String,
    pub pname: String,
    pub version: String,
    pub desc_score: f32,
    pub key_score: f32,
}

pub fn search_cache(strings: &Vec<String>, config: &AppConfig) -> Vec<SearchResult> {
    let cache = get_cache(&config).unwrap();
    let mut matching_results: Vec<SearchResult> = cache.package_iter().filter_map(|(key, result)| score_result(key, result, &strings)).collect();
    matching_results.sort_by(|a, b| b.desc_score.partial_cmp(&a.desc_score).unwrap());
    matching_results.sort_by(|a, b| b.key_score.partial_cmp(&a.key_score).unwrap());
    info!("{} matching results", matching_results.len());
    // info!("top result: {:?}", matching_results[0]);
    matching_results
}

fn score_result(key: &String, result: &CacheEntry, search_strings: &Vec<String>) -> Option<SearchResult> {
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



