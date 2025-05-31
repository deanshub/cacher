use std::path::Path;
use std::fs;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use glob::Pattern;
use anyhow::{Result, Context};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HintFile {
    #[serde(default)]
    pub default: DefaultSettings,
    #[serde(default)]
    pub commands: Vec<CommandHint>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct DefaultSettings {
    pub ttl: Option<u64>,
    #[serde(default)]
    pub include_env: HashSet<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandHint {
    pub pattern: String,
    pub ttl: Option<u64>,
    #[serde(default)]
    pub include_env: HashSet<String>,
    #[serde(default)]
    pub depends_on: Vec<Dependency>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    File {
        file: String,
    },
    Files {
        files: String,
    },
    Lines {
        lines: LinePattern,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LinePattern {
    pub file: String,
    pub pattern: String,
}

impl HintFile {
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read hint file: {}", path.display()))?;
        
        let hint_file: HintFile = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse hint file: {}", path.display()))?;
        
        Ok(hint_file)
    }
    
    pub fn find_matching_command(&self, command: &str) -> Option<&CommandHint> {
        self.commands.iter().find(|cmd| {
            match Pattern::new(&cmd.pattern) {
                Ok(pattern) => pattern.matches(command),
                Err(_) => cmd.pattern == command,
            }
        })
    }
    
    pub fn find_hint_file(start_dir: &Path) -> Option<Self> {
        let mut current_dir = Some(start_dir);
        
        while let Some(dir) = current_dir {
            let hint_file_path = dir.join(".cacher.yaml");
            if hint_file_path.exists() {
                return Self::from_file(&hint_file_path).ok();
            }
            
            current_dir = dir.parent();
        }
        
        None
    }
}

impl Dependency {
    pub fn get_files(&self, base_dir: &Path) -> Result<Vec<String>> {
        match self {
            Dependency::File { file } => {
                Ok(vec![file.clone()])
            },
            Dependency::Files { files } => {
                let pattern = files;
                let mut matches = Vec::new();
                
                for entry in glob::glob(&format!("{}/{}", base_dir.display(), pattern))? {
                    if let Ok(path) = entry {
                        if let Some(path_str) = path.to_str() {
                            matches.push(path_str.to_string());
                        }
                    }
                }
                
                Ok(matches)
            },
            Dependency::Lines { lines } => {
                Ok(vec![lines.file.clone()])
            }
        }
    }
    
    pub fn get_content_hash(&self, base_dir: &Path) -> Result<String> {
        use sha2::{Sha256, Digest};
        
        match self {
            Dependency::File { file } => {
                let path = base_dir.join(file);
                let content = fs::read(&path)
                    .with_context(|| format!("Failed to read file: {}", path.display()))?;
                
                let mut hasher = Sha256::new();
                hasher.update(&content);
                Ok(format!("{:x}", hasher.finalize()))
            },
            Dependency::Files { files } => {
                let mut combined_hash = String::new();
                
                for file in self.get_files(base_dir)? {
                    let path = Path::new(&file);
                    if path.exists() {
                        let content = fs::read(path)
                            .with_context(|| format!("Failed to read file: {}", path.display()))?;
                        
                        let mut hasher = Sha256::new();
                        hasher.update(&content);
                        combined_hash.push_str(&format!("{:x}", hasher.finalize()));
                    }
                }
                
                let mut final_hasher = Sha256::new();
                final_hasher.update(combined_hash);
                Ok(format!("{:x}", final_hasher.finalize()))
            },
            Dependency::Lines { lines } => {
                let path = base_dir.join(&lines.file);
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read file: {}", path.display()))?;
                
                let pattern = regex::Regex::new(&lines.pattern)
                    .with_context(|| format!("Invalid regex pattern: {}", lines.pattern))?;
                
                let mut matching_lines = String::new();
                for line in content.lines() {
                    if pattern.is_match(line) {
                        matching_lines.push_str(line);
                        matching_lines.push('\n');
                    }
                }
                
                let mut hasher = Sha256::new();
                hasher.update(matching_lines);
                Ok(format!("{:x}", hasher.finalize()))
            }
        }
    }
}
