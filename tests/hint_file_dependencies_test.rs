#[cfg(test)]
mod tests {
    use std::fs;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;
    use cacher::CommandCache;

    #[test]
    fn test_hint_file_dependencies() {
        // Create a temporary directory for the test
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();
        
        // Create a directory structure with various files
        fs::create_dir_all(temp_path.join("src/components")).unwrap();
        fs::create_dir_all(temp_path.join("docs")).unwrap();
        
        // Create files that should affect the cache
        fs::write(temp_path.join("package.json"), r#"{"name": "test-project", "version": "1.0.0"}"#).unwrap();
        fs::write(temp_path.join("src/index.js"), "console.log('Hello world');").unwrap();
        fs::write(temp_path.join("src/components/Button.js"), "export const Button = () => {};").unwrap();
        
        // Create files that should NOT affect the cache
        fs::write(temp_path.join("docs/README.md"), "# Documentation").unwrap();
        fs::write(temp_path.join("notes.txt"), "Some notes").unwrap();
        
        // Create a .cacher.yaml hint file that only includes certain files
        let hint_file_content = r#"
default:
  ttl: 3600

commands:
  - pattern: "npm run build"
    depends_on:
      - file: "package.json"
      - files: "src/**/*.js"
"#;
        
        fs::write(temp_path.join(".cacher.yaml"), hint_file_content).unwrap();
        
        // Change to the temp directory
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_path).unwrap();
        
        // Create a new CommandCache
        let cache = CommandCache::new();
        
        // Verify the hint file was loaded
        assert!(cache.get_hint_file().is_some());
        
        // Get the initial hash for our test command
        let command = "npm run build";
        let initial_hash = cache.generate_id(command);
        
        // PART 1: Modify a file that is NOT included in the hint file dependencies
        fs::write(temp_path.join("docs/README.md"), "# Updated Documentation").unwrap();
        fs::write(temp_path.join("notes.txt"), "Updated notes").unwrap();
        
        // Create a new cache to pick up the file changes
        let cache = CommandCache::new();
        
        // Get the hash after modifying non-included files
        let hash_after_non_included_changes = cache.generate_id(command);
        
        // The hash should NOT change
        assert_eq!(
            initial_hash, 
            hash_after_non_included_changes,
            "Hash should not change when modifying files not included in hint file dependencies"
        );
        
        // PART 2: Modify a file that IS included in the hint file dependencies
        // Sleep for a second to ensure file modification time changes
        thread::sleep(Duration::from_secs(1));
        fs::write(temp_path.join("package.json"), r#"{"name": "test-project", "version": "1.0.1"}"#).unwrap();
        
        // Create a new cache to pick up the file changes
        let cache = CommandCache::new();
        
        // Get the hash after modifying an included file
        let hash_after_included_changes = cache.generate_id(command);
        
        // The hash SHOULD change
        assert_ne!(
            initial_hash, 
            hash_after_included_changes,
            "Hash should change when modifying files included in hint file dependencies"
        );
        
        // PART 3: Modify a file that matches a glob pattern in the hint file
        // Sleep for a second to ensure file modification time changes
        thread::sleep(Duration::from_secs(1));
        fs::write(temp_path.join("src/components/Button.js"), "export const Button = (props) => {};").unwrap();
        
        // Create a new cache to pick up the file changes
        let cache = CommandCache::new();
        
        // Get the hash after modifying a file matching a glob pattern
        let hash_after_glob_changes = cache.generate_id(command);
        
        // The hash SHOULD change again
        assert_ne!(
            hash_after_included_changes, 
            hash_after_glob_changes,
            "Hash should change when modifying files matching glob patterns in hint file dependencies"
        );
        
        // Clean up
        std::env::set_current_dir(original_dir).unwrap();
    }
}
