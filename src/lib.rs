use std::collections::HashMap;
use std::process::Command;
use std::io::{self, Error, ErrorKind, Read, Write};
use std::fs::{self, File};
use std::path::PathBuf;
use sha2::{Sha256, Digest};
use dirs::cache_dir;
use std::time::{Duration, SystemTime};

pub struct CacheEntry {
    pub command: String,
    pub output: String,
    pub timestamp: SystemTime,
}

pub struct CommandCache {
    cache: HashMap<String, String>,
    cache_dir: PathBuf,
}

impl CommandCache {
    pub fn new() -> Self {
        let cache_dir = cache_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cacher");
        
        // Create cache directory if it doesn't exist
        let _ = fs::create_dir_all(&cache_dir);
        
        CommandCache {
            cache: HashMap::new(),
            cache_dir,
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
        hasher.update(command.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    pub fn get_cache_path(&self, id: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.cache", id))
    }
    
    pub fn save_to_disk(&self, command: &str, output: &str) -> io::Result<()> {
        let id = self.generate_id(command);
        let path = self.get_cache_path(&id);
        
        let entry = CacheEntry {
            command: command.to_string(),
            output: output.to_string(),
            timestamp: SystemTime::now(),
        };
        
        let json = format!(
            "{{\"command\":\"{}\",\"output\":\"{}\",\"timestamp\":{}}}",
            entry.command.replace("\"", "\\\""),
            entry.output.replace("\"", "\\\"").replace("\n", "\\n"),
            entry.timestamp.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        );
        
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        
        Ok(())
    }
    
    pub fn load_from_disk(&self, command: &str) -> io::Result<Option<String>> {
        let id = self.generate_id(command);
        let path = self.get_cache_path(&id);
        
        if !path.exists() {
            return Ok(None);
        }
        
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        // Simple parsing to extract output field from JSON
        if let Some(start) = contents.find("\"output\":\"") {
            if let Some(end) = contents[start + 10..].find("\"") {
                let output = &contents[start + 10..start + 10 + end];
                return Ok(Some(output.replace("\\n", "\n").replace("\\\"", "\"")));
            }
        }
        
        Err(Error::new(ErrorKind::InvalidData, "Invalid cache file format"))
    }
    
    pub fn execute_and_cache(&mut self, command: &str, ttl: Option<Duration>, force: bool) -> io::Result<String> {
        // If force is true, skip cache lookup
        if !force {
            // First check in-memory cache
            if let Some(output) = self.get(command) {
                return Ok(output.clone());
            }
            
            // Then check disk cache
            if let Ok(Some((output, timestamp))) = self.load_from_disk_with_timestamp(command) {
                // Check if cache is still valid based on TTL
                if let Some(ttl_duration) = ttl {
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
        
        // Parse command into program and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "Empty command"));
        }
        
        let program = parts[0];
        let args = &parts[1..];
        
        // Execute command
        let output = Command::new(program)
            .args(args)
            .output()?;
            
        if !output.status.success() {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Command failed with exit code: {:?}", output.status.code())
            ));
        }
        
        // Convert output to string
        let output_str = String::from_utf8_lossy(&output.stdout).to_string();
        
        // Cache the result in memory
        self.store(command, &output_str);
        
        // Cache the result on disk
        self.save_to_disk(command, &output_str)?;
        
        Ok(output_str)
    }
    
    pub fn load_from_disk_with_timestamp(&self, command: &str) -> io::Result<Option<(String, SystemTime)>> {
        let id = self.generate_id(command);
        let path = self.get_cache_path(&id);
        
        if !path.exists() {
            return Ok(None);
        }
        
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        // Extract output and timestamp from JSON
        let mut output = String::new();
        let mut timestamp = SystemTime::UNIX_EPOCH;
        
        if let Some(start) = contents.find("\"output\":\"") {
            if let Some(end) = contents[start + 10..].find("\"") {
                output = contents[start + 10..start + 10 + end]
                    .replace("\\n", "\n")
                    .replace("\\\"", "\"");
            }
        }
        
        if let Some(start) = contents.find("\"timestamp\":") {
            if let Some(end) = contents[start + 12..].find("}") {
                if let Ok(secs) = contents[start + 12..start + 12 + end].trim().parse::<u64>() {
                    timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
                }
            }
        }
        
        if output.is_empty() {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid cache file format"));
        }
        
        Ok(Some((output, timestamp)))
    }
    
    pub fn list_cached_commands(&self) -> io::Result<Vec<(String, SystemTime)>> {
        let mut entries = Vec::new();
        
        if !self.cache_dir.exists() {
            return Ok(entries);
        }
        
        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|ext| ext.to_str()) == Some("cache") {
                if let Ok(mut file) = File::open(&path) {
                    let mut contents = String::new();
                    if file.read_to_string(&mut contents).is_ok() {
                        // Simple parsing to extract command and timestamp fields from JSON
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
        
        // Sort by timestamp (newest first)
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        
        Ok(entries)
    }
    
    pub fn clear_cache(&self, command: Option<&str>) -> io::Result<usize> {
        let mut count = 0;
        
        if !self.cache_dir.exists() {
            return Ok(count);
        }
        
        if let Some(cmd) = command {
            // Clear specific command
            let id = self.generate_id(cmd);
            let path = self.get_cache_path(&id);
            
            if path.exists() {
                fs::remove_file(path)?;
                count = 1;
            }
        } else {
            // Clear all cache
            for entry in fs::read_dir(&self.cache_dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.extension().and_then(|ext| ext.to_str()) == Some("cache") {
                    fs::remove_file(path)?;
                    count += 1;
                }
            }
        }
        
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread::sleep;

    #[test]
    fn test_store_and_retrieve() {
        let mut cache = CommandCache::new();
        let command = "ls -la";
        let output = "file1\nfile2\nfile3";
        
        cache.store(command, output);
        
        assert_eq!(cache.get(command), Some(&output.to_string()));
    }

    #[test]
    fn test_retrieve_nonexistent() {
        let cache = CommandCache::new();
        let command = "ls -la";
        
        assert_eq!(cache.get(command), None);
    }
    
    #[test]
    fn test_execute_and_cache() {
        let mut cache = CommandCache::new();
        
        // Use a simple command that should work on any system
        let command = "echo hello";
        
        // First execution should run the command
        let result = cache.execute_and_cache(command, None, false);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello"));
        
        // Second execution should use the cache
        let cached_result = cache.execute_and_cache(command, None, false);
        assert!(cached_result.is_ok());
        assert_eq!(cached_result.unwrap(), output);
    }

    #[test]
    fn test_generate_id() {
        let cache = CommandCache::new();
        let command1 = "echo hello";
        let command2 = "echo world";
        
        // Same command should generate same id
        let id1 = cache.generate_id(command1);
        let id1_duplicate = cache.generate_id(command1);
        assert_eq!(id1, id1_duplicate);
        
        // Different commands should generate different ids
        let id2 = cache.generate_id(command2);
        assert_ne!(id1, id2);
        
        // ID should be a valid hex string
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(id1.len(), 64); // SHA-256 produces 32 bytes = 64 hex chars
    }
    
    #[test]
    fn test_disk_cache() {
        let cache = CommandCache::new();
        let command = "test_disk_cache_command";
        let output = "test_output";
        
        // Save to disk
        let save_result = cache.save_to_disk(command, output);
        assert!(save_result.is_ok());
        
        // Load from disk
        let load_result = cache.load_from_disk(command);
        assert!(load_result.is_ok());
        assert_eq!(load_result.unwrap(), Some(output.to_string()));
        
        // Clean up
        let id = cache.generate_id(command);
        let path = cache.get_cache_path(&id);
        let _ = fs::remove_file(path);
    }
    
    #[test]
    fn test_list_and_clear_cache() {
        let cache = CommandCache::new();
        
        // Add some test entries
        let commands = vec![
            "test_command_1",
            "test_command_2",
            "test_command_3",
        ];
        
        for cmd in &commands {
            cache.save_to_disk(cmd, "test_output").unwrap();
        }
        
        // List cache
        let entries = cache.list_cached_commands().unwrap();
        assert!(entries.len() >= commands.len());
        
        // Clear specific command
        let cleared = cache.clear_cache(Some(commands[0])).unwrap();
        assert_eq!(cleared, 1);
        
        // Verify it was cleared
        let entries_after = cache.list_cached_commands().unwrap();
        assert!(entries_after.len() < entries.len());
        
        // Clear all remaining test entries
        for cmd in &commands[1..] {
            let _ = cache.clear_cache(Some(cmd));
        }
    }
    
    #[test]
    fn test_ttl_and_force() {
        let mut cache = CommandCache::new();
        let command = "echo ttl_test";
        
        // First execution
        let result = cache.execute_and_cache(command, None, false);
        assert!(result.is_ok());
        
        // Force execution (should not use cache)
        let force_result = cache.execute_and_cache(command, None, true);
        assert!(force_result.is_ok());
        
        // With very short TTL (1ms)
        sleep(Duration::from_millis(10));
        let ttl_result = cache.execute_and_cache(command, Some(Duration::from_millis(1)), false);
        assert!(ttl_result.is_ok());
        
        // Clean up
        let _ = cache.clear_cache(Some(command));
    }
}
// Add the hint_file module
pub mod hint_file;
