#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::TempDir;
    use cacher::CommandCache;

    #[test]
    fn test_basic_artifact_caching() {
        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        println!("Test directory: {}", temp_path.display());
        
        // Create a source directory with some files
        let source_dir = temp_path.join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        
        // Create a .cacher file with directory artifact configuration
        let hint_file_content = r#"
commands:
  - pattern: "echo test_artifact"
    ttl: 60
    artifacts:
      - type: "directory"
        path: "source"
"#;
        
        fs::write(temp_path.join(".cacher"), hint_file_content).unwrap();
        
        // Change to the temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();
        
        // Create a new CommandCache
        let mut cache = CommandCache::new();
        
        // Execute the command for the first time
        let command = "echo test_artifact";
        let output1 = cache.execute_and_cache_with_artifacts(command, None, false).unwrap();
        assert_eq!(output1.trim(), "test_artifact");
        
        // Delete the source directory and its contents
        fs::remove_dir_all(&source_dir).unwrap();
        assert!(!source_dir.exists());
        
        // Execute the command again - it should restore the directory from cache
        let output2 = cache.execute_and_cache_with_artifacts(command, None, false).unwrap();
        assert_eq!(output2.trim(), "test_artifact");
        
        // Verify the directory was restored
        assert!(source_dir.exists());
        assert!(source_dir.join("file1.txt").exists());
        
        // Verify the content of the file
        assert_eq!(fs::read_to_string(source_dir.join("file1.txt")).unwrap(), "content1");
        
        // Clean up
        std::env::set_current_dir(original_dir).unwrap();
    }
}
