
use super::rule_collection::RuleCollection;
use hyper::Method;
use rand::Rng;
use regex::RegexSet;
use serde::Deserialize;
use std::path::PathBuf;
use std::str::FromStr;
use std::{error, fs, io};

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

lazy_static! {
    static ref HTTP_METHODS: Vec<String> = vec![
        String::from("GET"),
        String::from("OPTIONS"),
        String::from("POST"),
        String::from("PUT"),
        String::from("DELETE"),
        String::from("HEAD"),
        String::from("TRACE"),
        String::from("CONNECT"),
        String::from("PATCH"),
    ];
}

#[derive(Deserialize, Debug, Clone)]
pub struct Configuration {
    pub selected: usize,
    pub rule_collection: Vec<RuleCollection>,
    loaded_paths: Vec<PathBuf>,
}

impl Default for Configuration {
    fn default() -> Self {
       Configuration { selected:  0, rule_collection: vec![RuleCollection::default()], loaded_paths: vec![] } 
    }
}

impl Configuration {
    pub fn new(path_to_config: &PathBuf) -> Result<Configuration> {
        let mut rules = Configuration {
            selected: 0,
            rule_collection: Vec::new(),
            loaded_paths: Vec::new(),
        };
        rules.load_from_path(path_to_config).unwrap();
        if let Some(x) = rules.rule_collection.get_mut(0) {
            x.selected = true;
            Ok(rules)
        } else {
            Err("could not find rulecollections".into())
        }
    }

    pub fn toggle_rule(&mut self) {
        self.rule_collection[self.selected].active = !self.rule_collection[self.selected].active
    }

    pub fn select_prev(&mut self) {
        self.rule_collection[self.selected].selected = false;
        match self.selected {
            0 => self.selected = self.rule_collection.len() - 1,
            _ => self.selected -= 1,
        }
        self.rule_collection[self.selected].selected = true;
    }

    pub fn select_next(&mut self) {
        self.rule_collection[self.selected].selected = false;
        self.selected = (self.selected + 1) % self.rule_collection.len();
        self.rule_collection[self.selected].selected = true;
    }

    pub fn paths(&self) -> Vec<String> {
        self.loaded_paths
            .iter()
            .map(|e| String::from(e.to_str().unwrap()))
            .collect()
    }

    pub fn reload(&mut self) -> io::Result<()> {
        self.rule_collection = Vec::new();
        for path in self.loaded_paths.iter() {
            let f = std::fs::File::open(path)?;
            let d: Vec<RuleCollection> = serde_yaml::from_reader(f).ok().unwrap();
            for rule in d {
                self.rule_collection.push(rule)
            }
        }
        Ok(())
    }

    fn load_from_path(&mut self, path_to_config: &PathBuf) -> Result<()> {
        let abs_path_to_config = std::fs::canonicalize(path_to_config).unwrap();
        let allowed_file_extensions = vec!["yaml", "yml"];
        let regex_matcher = RegexSet::new(allowed_file_extensions)?;
        let mut entries: Vec<_> = fs::read_dir(abs_path_to_config)?
            .filter_map(|file| match file {
                Ok(file) => {
                    let path = file.path();
                    let extension = path.extension()?.to_str();
                    match extension {
                        Some(ext) if regex_matcher.is_match(ext) => Some(path),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect();
        entries.sort();
        for path in entries.iter() {
            let f = std::fs::File::open(path).unwrap();
            let d: Vec<RuleCollection> = serde_yaml::from_reader(f)?;
            for rule in d {
                self.rule_collection.push(rule)
            }
            self.loaded_paths.push(path.clone());
        }
        Ok(())
    }

    pub fn active_matching_rules(&mut self, uri: &str, method: &Method, body: &str) -> Vec<usize> {
        let mut rng = rand::thread_rng();

        let rule_path_names: Vec<String> = self
            .rule_collection
            .iter()
            .map(|rule| rule.path.to_owned())
            .collect();

        let set = RegexSet::new(rule_path_names).unwrap();

        set.matches(uri)
            .into_iter()
            .filter(|i| {
                if !self.rule_collection[*i].active {
                    return false;
                }

                let mut probability_matches = true;
                if let Some(prob) = self.rule_collection[*i].match_with_prob {
                    probability_matches = rng.gen_range(0.0, 1.0) < prob;
                }

                let mut body_matches = true;
                if let Some(match_body) = &self.rule_collection[*i].match_body_contains {
                    body_matches = body.contains(match_body);
                }

                let method_matches = self.rule_collection[*i]
                    .match_methods
                    .as_ref()
                    .unwrap_or(&HTTP_METHODS)
                    .iter()
                    .map(|s| Method::from_str(s).unwrap())
                    .collect::<Vec<Method>>()
                    .contains(method);

                probability_matches && body_matches && method_matches
            })
            .collect()
    }

    pub fn clone_rule(&mut self, idx: usize) -> RuleCollection {
        self.rule_collection.get_mut(idx).unwrap().clone()
    }
}

