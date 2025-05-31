use std::collections::HashMap;
use std::io::{self, Error, ErrorKind, Read, Write};
use std::fs::{self, File};
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use dirs::cache_dir;
use std::time::{Duration, SystemTime};
use std::env;
use crate::hint_file::{HintFile, Dependency};

pub struct CacheEntry {
    pub command: String,
    pub output: String,
    pub timestamp: SystemTime,
}

pub struct CommandCache {
    cache: HashMap<String, String>,
    cache_dir: PathBuf,
    hint_file: Option<HintFile>,
    current_dir: PathBuf,
}

impl CommandCache {
    pub fn new() -> Self {
        // Get cache directory
        let mut cache_dir = cache_dir().unwrap_or_else(|| PathBuf::from("."));
        cache_dir.push("cacher");
        
        // Create cache directory if it doesn't exist
        let _ = fs::create_dir_all(&cache_dir);
        
        // Get current directory
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        
        // Try to load hint file
        let hint_file = HintFile::find_hint_file(&current_dir);
        
        CommandCache {
            cache: HashMap::new(),
            cache_dir,
            hint_file,
            current_dir,
        }
    }

    pub fn store(&mut self, command: &str, output: &str) {
        self.cache.insert(command.to_string(), output.to_string());
    }

    pub fn get(&self, command: &str) -> Option<&String> {
        self.cache.get(command)
    }
    
    pub fn generate_id(&self, command: &str) -> String {
        let mut hasher = Sha256::new();
        
        // Add the command itself to the hash
        hasher.update(command.as_bytes());
        
        // If we have a hint file, check for command-specific settings
        if let Some(hint_file) = &self.hint_file {
            // Check if there's a matching command pattern
            if let Some(command_hint) = hint_file.find_matching_command(command) {
                // Include specified environment variables in the hash
                for env_var in &command_hint.include_env {
                    if let Ok(value) = env::var(env_var) {
                        hasher.update(format!("{}={}", env_var, value).as_bytes());
                    }
                }
                
                // Include file dependencies in the hash
                for dependency in &command_hint.depends_on {
                    match dependency {
                        Dependency::File { file } => {
                            let path = self.current_dir.join(file);
                            if path.exists() {
                                if let Ok(metadata) = fs::metadata(&path) {
                                    if let Ok(modified) = metadata.modified() {
                                        if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                                            hasher.update(format!("{}={}", file, duration.as_secs()).as_bytes());
                                        }
                                    }
                                }
                            }
                        },
                        Dependency::Files { files } => {
                            // Use glob pattern to find matching files
                            if let Ok(entries) = glob::glob(&format!("{}/{}", self.current_dir.display(), files)) {
                                for entry in entries {
                                    if let Ok(path) = entry {
                                        if let Ok(metadata) = fs::metadata(&path) {
                                            if let Ok(modified) = metadata.modified() {
                                                if let Ok(duration) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                                                    if let Some(path_str) = path.to_str() {
                                                        hasher.update(format!("{}={}", path_str, duration.as_secs()).as_bytes());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        Dependency::Lines { lines } => {
                            let path = self.current_dir.join(&lines.file);
                            if path.exists() {
                                if let Ok(content) = fs::read_to_string(&path) {
                                    if let Ok(regex) = regex::Regex::new(&lines.pattern) {
                                        let mut matching_lines = String::new();
                                        for line in content.lines() {
                                            if regex.is_match(line) {
                                                matching_lines.push_str(line);
                                                matching_lines.push('\n');
                                            }
                                        }
                                        hasher.update(matching_lines.as_bytes());
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // No specific command match, use default environment variables
                for env_var in &hint_file.default.include_env {
                    if let Ok(value) = env::var(env_var) {
                        hasher.update(format!("{}={}", env_var, value).as_bytes());
                    }
                }
            }
        }
        
        format!("{:x}", hasher.finalize())
    }
    
    pub fn get_cache_path(&self, id: &str) -> PathBuf {
        let cache_dir = self.cache_dir.join(id);
        fs::create_dir_all(&cache_dir).unwrap_or_else(|_| {});
        cache_dir
    }
    
    pub fn get_stdout_path(&self, id: &str) -> PathBuf {
        self.get_cache_path(id).join("stdout")
    }
    
    pub fn get_metadata_path(&self, id: &str) -> PathBuf {
        self.get_cache_path(id).join("metadata.json")
    }
    
    pub fn save_to_disk(&self, command: &str, output: &str) -> io::Result<()> {
        let id = self.generate_id(command);
        
        // Create cache directory for this command
        let _ = self.get_cache_path(&id);
        
        // Save stdout to a separate file
        let stdout_path = self.get_stdout_path(&id);
        let mut stdout_file = File::create(stdout_path)?;
        stdout_file.write_all(output.as_bytes())?;
        
        // Save metadata to a JSON file
        let metadata_path = self.get_metadata_path(&id);
        let metadata = format!(
            "{{\"command\":\"{}\",\"timestamp\":{}}}",
            command.replace("\"", "\\\""),
            SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        );
        
        let mut metadata_file = File::create(metadata_path)?;
        metadata_file.write_all(metadata.as_bytes())?;
        
        Ok(())
    }
    
    pub fn load_from_disk(&self, command: &str) -> io::Result<Option<String>> {
        let id = self.generate_id(command);
        let stdout_path = self.get_stdout_path(&id);
        
        if !stdout_path.exists() {
            return Ok(None);
        }
        
        let mut file = File::open(stdout_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        Ok(Some(contents))
    }
    
    pub fn execute_command(&self, command: &str) -> io::Result<String> {
        // Parse command into program and arguments
        let mut parts = command.split_whitespace();
        let program = parts.next().ok_or_else(|| {
            Error::new(ErrorKind::InvalidInput, "Empty command")
        })?;
        
        let args: Vec<&str> = parts.collect();
        
        // Execute command
        let output = std::process::Command::new(program)
            .args(&args)
            .output()
            .map_err(|e| {
                Error::new(ErrorKind::Other, format!("Failed to execute command: {}", e))
            })?;
        
        if !output.status.success() {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Command failed with exit code {}: {}",
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stderr)
                )
            ));
        }
        
        let output_str = String::from_utf8_lossy(&output.stdout).to_string();
        
        Ok(output_str)
    }
    
    pub fn execute_and_cache(&mut self, command: &str, ttl: Option<Duration>, force: bool) -> io::Result<String> {
        if !force {
            // First check in-memory cache
            if let Some(output) = self.get(command) {
                return Ok(output.clone());
            }
            
            // Then check disk cache
            if let Ok(Some((output, timestamp))) = self.load_from_disk_with_timestamp(command) {
                // Get TTL from hint file if available
                let effective_ttl = self.get_effective_ttl(command, ttl);
                
                // Check if cache is still valid based on TTL
                if let Some(ttl_duration) = effective_ttl {
                    if let Ok(age) = SystemTime::now().duration_since(timestamp) {
                        if age > ttl_duration {
                            // Cache is expired, don't use it
                        } else {
                            // Cache is still valid
                            self.store(command, &output);
                            return Ok(output);
                        }
                    }
                } else {
                    // No TTL specified, use cache regardless of age
                    self.store(command, &output);
                    return Ok(output);
                }
            }
        }
        
        // Execute command and cache result
        let output = self.execute_command(command)?;
        self.store(command, &output);
        self.save_to_disk(command, &output)?;
        
        Ok(output)
    }
    
    // Helper method to get effective TTL from hint file or fallback to provided TTL
    pub fn get_effective_ttl(&self, command: &str, default_ttl: Option<Duration>) -> Option<Duration> {
        if let Some(hint_file) = &self.hint_file {
            // Check for command-specific TTL
            if let Some(command_hint) = hint_file.find_matching_command(command) {
                if let Some(ttl_seconds) = command_hint.ttl {
                    return Some(Duration::from_secs(ttl_seconds));
                }
            }
            
            // Fall back to default TTL from hint file
            if let Some(ttl_seconds) = hint_file.default.ttl {
                return Some(Duration::from_secs(ttl_seconds));
            }
        }
        
        // Fall back to provided TTL
        default_ttl
    }
    
    pub fn load_from_disk_with_timestamp(&self, command: &str) -> io::Result<Option<(String, SystemTime)>> {
        let id = self.generate_id(command);
        let stdout_path = self.get_stdout_path(&id);
        let metadata_path = self.get_metadata_path(&id);
        
        if !stdout_path.exists() || !metadata_path.exists() {
            return Ok(None);
        }
        
        // Read stdout content
        let mut stdout_file = File::open(stdout_path)?;
        let mut stdout_content = String::new();
        stdout_file.read_to_string(&mut stdout_content)?;
        
        // Read metadata
        let mut metadata_file = File::open(metadata_path)?;
        let mut metadata_content = String::new();
        metadata_file.read_to_string(&mut metadata_content)?;
        
        // Parse timestamp from metadata
        let mut timestamp = SystemTime::UNIX_EPOCH;
        if let Some(start) = metadata_content.find("\"timestamp\":") {
            if let Some(end) = metadata_content[start + 12..].find("}") {
                if let Ok(secs) = metadata_content[start + 12..start + 12 + end].trim().parse::<u64>() {
                    timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
                }
            }
        }
        
        Ok(Some((stdout_content, timestamp)))
    }
    
    pub fn list_cached_commands(&self) -> io::Result<Vec<(String, SystemTime)>> {
        let mut entries = Vec::new();
        
        if !self.cache_dir.exists() {
            return Ok(entries);
        }
        
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let cache_dir = entry.path();
            
            if cache_dir.is_dir() {
                let metadata_path = cache_dir.join("metadata.json");
                if metadata_path.exists() {
                    if let Ok(mut file) = File::open(&metadata_path) {
                        let mut contents = String::new();
                        if file.read_to_string(&mut contents).is_ok() {
                            // Parse command and timestamp from metadata
                            let mut command = String::new();
                            let mut timestamp = SystemTime::UNIX_EPOCH;
                            
                            if let Some(start) = contents.find("\"command\":\"") {
                                if let Some(end) = contents[start + 11..].find("\"") {
                                    command = contents[start + 11..start + 11 + end]
                                        .replace("\\\"", "\"")
                                        .to_string();
                                }
                            }
                            
                            if let Some(start) = contents.find("\"timestamp\":") {
                                if let Some(end) = contents[start + 12..].find("}") {
                                    if let Ok(secs) = contents[start + 12..start + 12 + end].trim().parse::<u64>() {
                                        timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
                                    }
                                }
                            }
                            
                            if !command.is_empty() {
                                entries.push((command, timestamp));
                            }
                        }
                    }
                }
            }
        }
        
        entries.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by timestamp, newest first
        Ok(entries)
    }
    
    pub fn clear_cache(&mut self, command: Option<&str>) -> io::Result<()> {
        if !self.cache_dir.exists() {
            return Ok(());
        }
        
        match command {
            Some(cmd) => {
                // Clear specific command
                let id = self.generate_id(cmd);
                let cache_dir = self.get_cache_path(&id);
                if cache_dir.exists() {
                    fs::remove_dir_all(cache_dir)?;
                }
                self.cache.remove(cmd);
            },
            None => {
                // Clear all cache
                fs::remove_dir_all(&self.cache_dir)?;
                fs::create_dir_all(&self.cache_dir)?;
                self.cache.clear();
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_store_and_retrieve() {
        let mut cache = CommandCache::new();
        let command = "echo hello";
        let output = "hello\n";
        
        cache.store(command, output);
        assert_eq!(cache.get(command), Some(&output.to_string()));
    }
    
    #[test]
    fn test_retrieve_nonexistent() {
        let cache = CommandCache::new();
        let command = "echo nonexistent";
        
        assert_eq!(cache.get(command), None);
    }
    
    #[test]
    fn test_generate_id() {
        let cache = CommandCache::new();
        let command = "echo hello";
        
        let id1 = cache.generate_id(command);
        let id2 = cache.generate_id(command);
        
        assert_eq!(id1, id2);
        assert!(!id1.is_empty());
    }
    
    #[test]
    fn test_disk_cache() {
        let cache = CommandCache::new();
        let command = "test_disk_cache_command";
        let output = "test output";
        
        // Save to disk
        cache.save_to_disk(command, output).unwrap();
        
        // Load from disk
        let loaded = cache.load_from_disk(command).unwrap();
        assert_eq!(loaded, Some(output.to_string()));
    }
    
    #[test]
    fn test_execute_and_cache() {
        let mut cache = CommandCache::new();
        let command = "echo test_execute";
        
        // Execute and cache
        let result = cache.execute_and_cache(command, None, false);
        assert!(result.is_ok());
        
        // Check in-memory cache
        assert!(cache.get(command).is_some());
        
        // Check disk cache
        let loaded = cache.load_from_disk(command).unwrap();
        assert!(loaded.is_some());
    }
    
    #[test]
    fn test_ttl_and_force() {
        let mut cache = CommandCache::new();
        let command = "echo ttl_test";
        
        // Execute and cache with short TTL
        let result1 = cache.execute_and_cache(command, Some(Duration::from_secs(1)), false).unwrap();
        
        // Wait for TTL to expire
        std::thread::sleep(Duration::from_secs(2));
        
        // Execute again, should re-execute due to expired TTL
        let result2 = cache.execute_and_cache(command, Some(Duration::from_secs(1)), false).unwrap();
        
        assert_eq!(result1, result2);
        
        // Force execution
        let result3 = cache.execute_and_cache(command, None, true).unwrap();
        assert_eq!(result2, result3);
    }
    
    #[test]
    fn test_list_and_clear_cache() {
        let mut cache = CommandCache::new();
        let command = "echo list_test";
        
        // Execute and cache
        let _ = cache.execute_and_cache(command, None, false);
        
        // List cached commands
        let entries = cache.list_cached_commands().unwrap();
        assert!(!entries.is_empty());
        
        // Clear cache
        let _ = cache.clear_cache(Some(command));
    }
}
// Add the hint_file module
pub mod hint_file;

impl CommandCache {
    /// Reload the hint file from the current directory
    ///
    /// This is useful when the hint file has been modified or when
    /// the current directory has changed.
    pub fn reload_hint_file(&mut self) {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.current_dir = current_dir;
        self.hint_file = HintFile::find_hint_file(&self.current_dir);
    }
    
    /// Get a reference to the current hint file, if one is loaded
    ///
    /// # Returns
    ///
    /// An Option containing a reference to the HintFile, or None if no hint file is loaded
    pub fn get_hint_file(&self) -> Option<&HintFile> {
        self.hint_file.as_ref()
    }
}
