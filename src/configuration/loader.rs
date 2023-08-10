use std::fs::DirEntry;
use std::path::PathBuf;
use regex::RegexSet;

use std::{fs};
use thiserror::Error;

use super::nconfiguration::RuleSet;


#[derive(Error, Debug)]
pub enum DeserializationError {
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
    pub(crate) extensions: Vec<String>
}

impl YamlFileLoader {
    fn deserialize_file(
        &self,
        f: Result<DirEntry, std::io::Error>,
    ) -> Result<Vec<RuleSet>, DeserializationError> {
        let regex_matcher = RegexSet::new(&self.extensions)?;
        let p = f?.path().clone();
        let c = p.clone();
        let ext = p
            .extension()
            .ok_or(DeserializationError::MissingExtension)?;
        let matches_allowed_ext =
            regex_matcher.is_match(ext.to_str().ok_or(DeserializationError::MissingExtension)?);

        if matches_allowed_ext {
            let file_buffer = std::fs::File::open(p)?;
            let mut content: Vec<RuleSet> = serde_yaml::from_reader(file_buffer)?;
            for rule in &mut content {
                match rule {
                    RuleSet::Rule(r) => {
                        r.path = String::from(c.to_str().unwrap());
                    },
                }
            }
            Ok(content)
        } else {
            Err(DeserializationError::ForbiddenExtension)
        }
    }

    pub fn load_from_directories(
        &self, directories: &Vec<PathBuf>,
    ) -> Result<Vec<RuleSet>, DeserializationError> {
        let mut dir_contents: Vec<RuleSet> = Vec::new();

        for path in directories {
            let absolute_path = std::fs::canonicalize(path)?;
            let entries = fs::read_dir(absolute_path)?;
            for file in entries {
                let deserialized_rules = self.deserialize_file(file)?;
                dir_contents.extend(deserialized_rules);
            }
        }
        Ok(dir_contents)
    }
}
