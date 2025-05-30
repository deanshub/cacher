use std::collections::HashMap;
use std::process::Command;
use std::io::{self, Error, ErrorKind};

pub struct CommandCache {
    cache: HashMap<String, String>,
}

impl CommandCache {
    pub fn new() -> Self {
        CommandCache {
            cache: HashMap::new(),
        }
    }

    pub fn store(&mut self, command: &str, output: &str) {
        self.cache.insert(command.to_string(), output.to_string());
    }

    pub fn get(&self, command: &str) -> Option<&String> {
        self.cache.get(command)
    }
    
    pub fn execute_and_cache(&mut self, command: &str) -> io::Result<String> {
        // Check if command is already cached
        if let Some(output) = self.get(command) {
            return Ok(output.clone());
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
        
        // Cache the result
        self.store(command, &output_str);
        
        Ok(output_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let result = cache.execute_and_cache(command);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello"));
        
        // Second execution should use the cache
        let cached_result = cache.execute_and_cache(command);
        assert!(cached_result.is_ok());
        assert_eq!(cached_result.unwrap(), output);
    }
}
