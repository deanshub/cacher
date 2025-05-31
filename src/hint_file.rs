use std::path::Path;
use std::fs;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};
use glob::Pattern;
use anyhow::{Result, Context};

/// Represents a .cacher hint file that configures caching behavior
///
/// The hint file allows users to customize how caching works for specific commands,
/// including TTL settings, environment variables to include in the hash, and file
/// dependencies that should invalidate the cache when changed.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HintFile {
    /// Default settings that apply to all commands
    #[serde(default)]
    pub default: DefaultSettings,
    
    /// Command-specific settings that override defaults
    #[serde(default)]
    pub commands: Vec<CommandHint>,
}

/// Default settings that apply to all commands
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct DefaultSettings {
    /// Default time-to-live in seconds for cached entries
    pub ttl: Option<u64>,
    
    /// Environment variables to include in the cache key
    #[serde(default)]
    pub include_env: HashSet<String>,
}

/// Configuration for a specific command pattern
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommandHint {
    /// Glob pattern to match commands
    pub pattern: String,
    
    /// Time-to-live in seconds for this command
    pub ttl: Option<u64>,
    
    /// Environment variables to include in the cache key
    #[serde(default)]
    pub include_env: HashSet<String>,
    
    /// Dependencies that should invalidate the cache when changed
    #[serde(default)]
    pub depends_on: Vec<Dependency>,
}

/// Types of dependencies that can invalidate the cache
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Dependency {
    /// A single file dependency
    File {
        file: String,
    },
    /// A glob pattern matching multiple files
    Files {
        files: String,
    },
    /// Specific lines in a file matched by a regex pattern
    Lines {
        lines: LinePattern,
    },
}

/// Configuration for matching specific lines in a file
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LinePattern {
    /// Path to the file to match lines in
    pub file: String,
    
    /// Regex pattern to match lines
    pub pattern: String,
}

impl HintFile {
    /// Load a hint file from the specified path
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the hint file
    ///
    /// # Returns
    ///
    /// A Result containing the parsed HintFile or an error
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read hint file: {}", path.display()))?;
        
        let hint_file: HintFile = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse hint file: {}", path.display()))?;
        
        Ok(hint_file)
    }
    
    /// Find a command hint that matches the given command
    ///
    /// # Arguments
    ///
    /// * `command` - The command to match
    ///
    /// # Returns
    ///
    /// An Option containing the matching CommandHint, or None if no match is found
    pub fn find_matching_command(&self, command: &str) -> Option<&CommandHint> {
        self.commands.iter().find(|cmd| {
            match Pattern::new(&cmd.pattern) {
                Ok(pattern) => pattern.matches(command),
                Err(_) => cmd.pattern == command,
            }
        })
    }
    
    /// Find a hint file by searching up from the given directory
    ///
    /// Searches for a .cacher.yaml file in the given directory and its parents
    ///
    /// # Arguments
    ///
    /// * `start_dir` - Directory to start searching from
    ///
    /// # Returns
    ///
    /// An Option containing the parsed HintFile, or None if no hint file is found
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
    /// Get all files matching this dependency
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Base directory for resolving relative paths
    ///
    /// # Returns
    ///
    /// A Result containing a vector of file paths
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
    
    /// Calculate a hash of the content for this dependency
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Base directory for resolving relative paths
    ///
    /// # Returns
    ///
    /// A Result containing the hash as a hex string
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
            Dependency::Files { files: _ } => {
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
                
                // Use a default pattern if the regex is invalid
                let pattern = match regex::Regex::new(&lines.pattern) {
                    Ok(p) => p,
                    Err(e) => {
                        // Log the error but don't fail completely
                        eprintln!("Warning: Invalid regex pattern '{}': {}", lines.pattern, e);
                        // Use a pattern that matches nothing
                        regex::Regex::new(r"^$").unwrap()
                    }
                };
                
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
