use std::io::BufReader;


use std::fs::*;
use std::io::BufRead;
use std::path::PathBuf;

use super::HashSet;
use super::model::{UserId};

pub struct SearchResult {
    pub user_id: UserId,
    pub full_text: String,
}

pub fn pretty_search_result(text:&str, terms: &Vec<String>) -> String {
    let mut res = String::from(text);
    for term in terms {
        let bolded_term = format!("<b>{}</b>", term);
        res = res.replace(term, &bolded_term);
    }
    res
}

pub fn terms_for_search(text:&str) -> Vec<String> {
    let mut terms : Vec<String> = Vec::new();
    
    for word in text.to_lowercase().split_whitespace() {
        terms.push(String::from(word.trim()));
    }

    return terms
}

pub fn search(paths:Vec<PathBuf>, terms: &Vec<String>, user_ids:&HashSet<UserId>) -> Vec<SearchResult> {
    let mut results : Vec<SearchResult> = Vec::new();

    for path in paths {
        // println!("opening path {:?}", path);
        let file = File::open(path).unwrap();
        let reader = BufReader::new(file);
        let lines = reader.lines();

        for line_result in lines {
            let line = line_result.expect("attempted to read a line in model").to_lowercase();
            let at = line.find(" ").unwrap();
            let (num, text) = line.split_at(at);
            let maybe_user_id : Option<UserId> = num.parse().ok();
            if let Some(user_id) = maybe_user_id {
                if user_ids.contains(&user_id) {
                    let lowercase_text = text.to_lowercase(); 
                    let ok = terms.iter().all(|t| lowercase_text.contains(t));
                    if ok {
                        let result = SearchResult { 
                            user_id: user_id, 
                            full_text: lowercase_text 
                        };
                        // add context
                        results.push(result);
                    }
                }
            }
        }
    }
    
    return results
}