use regex::RegexSet;
use std::fs::DirEntry;
use std::path::PathBuf;

use std::fs;
use thiserror::Error;

use super::ruleset::RuleSet;

#[derive(Error, Debug)]
pub enum DeserializationError {
    #[allow(dead_code)]
    #[error("Could not read directory")]
    ReadDirectory(),
    #[error("could not parse regex")]
    Regex(#[from] regex::Error),
    #[error("using forbidden extension")]
    ForbiddenExtension,
    #[error("has no extension")]
    MissingExtension,
    #[error("could not read file")]
    IO(#[from] std::io::Error),
    #[error("could not parse yaml")]
    YamlParse(#[from] serde_yaml::Error),
}

pub struct YamlFileLoader {
    pub extensions: Vec<String>,
}

impl YamlFileLoader {
    fn deserialize_file(
        &self,
        f: Result<DirEntry, std::io::Error>,
    ) -> Result<Vec<RuleSet>, DeserializationError> {
        let regex_matcher = RegexSet::new(&self.extensions)?;
        let p = f?.path();
        let c = p.clone();
        
        // Match against full filename, not just extension
        let filename = p
            .file_name()
            .and_then(|f| f.to_str())
            .ok_or(DeserializationError::MissingExtension)?;
        
        let matches_allowed_ext = regex_matcher.is_match(filename);

        if matches_allowed_ext {
            let file_buffer = std::fs::File::open(p)?;
            let mut content: Vec<RuleSet> =
                serde_yaml::from_reader(file_buffer)?;
            for rule in &mut content {
                match rule {
                    RuleSet::Rule(r) => {
                        r.path = String::from(c.to_str().unwrap());
                    }
                }
            }
            Ok(content)
        } else {
            Err(DeserializationError::ForbiddenExtension)
        }
    }

    pub fn load_from_directories_with_errors(
        &self,
        directories: &[PathBuf],
    ) -> (Vec<RuleSet>, Vec<(String, String)>) {
        let mut dir_contents: Vec<RuleSet> = Vec::new();
        let mut errors: Vec<(String, String)> = Vec::new();

        let mut all_files = Vec::new();
        for p in directories {
            let read_dir = fs::read_dir(p);
            match read_dir {
                Ok(dir_entries) => {
                    for entry in dir_entries.flatten() {
                        all_files.push(entry);
                    }
                }
                Err(e) => {
                    errors.push((p.to_string_lossy().to_string(), format!("ReadDirectory: {}", e)));
                }
            }
        }

        all_files.sort_by_key(|f| f.path());

        for file in all_files {
            let path_str = file.path().to_string_lossy().to_string();
            match self.deserialize_file(Ok(file)) {
                Ok(rules) => {
                    log::info!("deserialized rules: {:?}", rules);
                    dir_contents.extend(rules);
                }
                Err(DeserializationError::ForbiddenExtension) => {
                    // Skip files with wrong extension silently
                    continue;
                }
                Err(e) => {
                    log::error!("PARSE_ERROR file='{}' error='{}'", path_str, e);
                    errors.push((path_str, format!("{}", e)));
                }
            }
        }
        (dir_contents, errors)
    }
}
