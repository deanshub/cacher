#[cfg(test)]
mod tests {
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;
    use cacher::CommandCache;

    #[test]
    fn test_directory_artifact_caching() {
        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create a source directory with some files
        let source_dir = temp_path.join("source");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("file1.txt"), "content1").unwrap();
        fs::write(source_dir.join("file2.txt"), "content2").unwrap();
        
        // Create a subdirectory with files
        let sub_dir = source_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("file3.txt"), "content3").unwrap();
        
        // Create a .cacher.yaml file with directory artifact configuration
        let hint_file_content = r#"
commands:
  - pattern: "echo test_artifact"
    ttl: 60
    artifacts:
      - type: "directory"
        path: "source"
"#;
        
        fs::write(temp_path.join(".cacher.yaml"), hint_file_content).unwrap();
        
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
        assert!(source_dir.join("file2.txt").exists());
        assert!(source_dir.join("subdir").exists());
        assert!(source_dir.join("subdir/file3.txt").exists());
        
        // Verify the content of the files
        assert_eq!(fs::read_to_string(source_dir.join("file1.txt")).unwrap(), "content1");
        assert_eq!(fs::read_to_string(source_dir.join("file2.txt")).unwrap(), "content2");
        assert_eq!(fs::read_to_string(source_dir.join("subdir/file3.txt")).unwrap(), "content3");
        
        // Modify the source directory
        fs::write(source_dir.join("file1.txt"), "modified1").unwrap();
        fs::write(source_dir.join("new_file.txt"), "new_content").unwrap();
        
        // Sleep to ensure modification time changes
        thread::sleep(Duration::from_secs(1));
        
        // Force execution to update the cache
        let output3 = cache.execute_and_cache_with_artifacts(command, None, true).unwrap();
        assert_eq!(output3.trim(), "test_artifact");
        
        // Delete the source directory again
        fs::remove_dir_all(&source_dir).unwrap();
        assert!(!source_dir.exists());
        
        // Execute the command again - it should restore the updated directory
        let output4 = cache.execute_and_cache_with_artifacts(command, None, false).unwrap();
        assert_eq!(output4.trim(), "test_artifact");
        
        // Verify the updated directory was restored
        assert!(source_dir.exists());
        assert!(source_dir.join("file1.txt").exists());
        assert!(source_dir.join("new_file.txt").exists());
        
        // Verify the content of the modified files
        assert_eq!(fs::read_to_string(source_dir.join("file1.txt")).unwrap(), "modified1");
        assert_eq!(fs::read_to_string(source_dir.join("new_file.txt")).unwrap(), "new_content");
        
        // Clean up
        std::env::set_current_dir(original_dir).unwrap();
    }
}
