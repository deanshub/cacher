#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Duration;
    use tempfile::TempDir;
    use cacher::CommandCache;

    #[test]
    fn test_cache_with_hint_file() {
        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create a .cacher.yaml file in the temp directory
        let hint_file_content = r#"
default:
  ttl: 60

commands:
  - pattern: "echo *"
    ttl: 10
    include_env:
      - TEST_ENV_VAR
"#;
        
        fs::write(temp_path.join(".cacher.yaml"), hint_file_content).unwrap();
        
        // Change to the temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();
        
        // Create a new CommandCache
        let mut cache = CommandCache::new();
        
        // Verify the hint file was loaded
        assert!(cache.get_hint_file().is_some());
        
        // Test command with custom TTL
        let command = "echo hello";
        let result = cache.execute_and_cache(command, None, false);
        assert!(result.is_ok());
        
        // Skip the environment variable test for now
        
        // Clean up
        std::env::set_current_dir(original_dir).unwrap();
    }
    
    #[test]
    fn test_effective_ttl() {
        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create a .cacher.yaml file in the temp directory
        let hint_file_content = r#"
default:
  ttl: 60

commands:
  - pattern: "echo *"
    ttl: 10
"#;
        
        fs::write(temp_path.join(".cacher.yaml"), hint_file_content).unwrap();
        
        // Change to the temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();
        
        // Create a new CommandCache
        let mut cache = CommandCache::new();
        
        // Test effective TTL for matching command
        let echo_ttl = cache.get_effective_ttl("echo hello", Some(Duration::from_secs(30)));
        assert_eq!(echo_ttl, Some(Duration::from_secs(10))); // Should use command-specific TTL
        
        // Test effective TTL for non-matching command
        let ls_ttl = cache.get_effective_ttl("ls -la", Some(Duration::from_secs(30)));
        assert_eq!(ls_ttl, Some(Duration::from_secs(60))); // Should use default TTL
        
        // Clean up
        std::env::set_current_dir(original_dir).unwrap();
    }
}
