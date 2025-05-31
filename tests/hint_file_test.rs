#[cfg(test)]
mod tests {
    use std::path::Path;
    use cacher::hint_file::{HintFile, Dependency};

    #[test]
    fn test_load_default_only() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/default_only.cacher.yaml")).unwrap();
        
        assert_eq!(hint_file.default.ttl, Some(3600));
        assert!(hint_file.default.include_env.is_empty());
        assert!(hint_file.commands.is_empty());
    }

    #[test]
    fn test_load_with_env_vars() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/with_env_vars.cacher.yaml")).unwrap();
        
        assert_eq!(hint_file.default.ttl, Some(3600));
        assert_eq!(hint_file.default.include_env.len(), 2);
        assert!(hint_file.default.include_env.contains(&"PATH".to_string()));
        assert!(hint_file.default.include_env.contains(&"USER".to_string()));
    }

    #[test]
    fn test_load_command_patterns() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/command_patterns.cacher.yaml")).unwrap();
        
        assert_eq!(hint_file.commands.len(), 2);
        
        let ls_command = hint_file.commands.iter().find(|c| c.pattern == "ls *").unwrap();
        assert_eq!(ls_command.ttl, Some(60));
        
        let git_command = hint_file.commands.iter().find(|c| c.pattern == "git status").unwrap();
        assert_eq!(git_command.ttl, Some(300));
    }

    #[test]
    fn test_load_file_dependencies() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/file_dependencies.cacher.yaml")).unwrap();
        
        let git_command = hint_file.commands.iter().find(|c| c.pattern == "git status").unwrap();
        assert_eq!(git_command.depends_on.len(), 2);
        
        let file_deps: Vec<&Dependency> = git_command.depends_on.iter()
            .filter(|d| matches!(d, Dependency::File { .. }))
            .collect();
        assert_eq!(file_deps.len(), 2);
    }

    #[test]
    fn test_load_glob_patterns() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/glob_patterns.cacher.yaml")).unwrap();
        
        let npm_command = hint_file.commands.iter().find(|c| c.pattern == "npm run *").unwrap();
        let webpack_command = hint_file.commands.iter().find(|c| c.pattern == "webpack *").unwrap();
        
        assert_eq!(npm_command.depends_on.len(), 1);
        assert_eq!(webpack_command.depends_on.len(), 2);
        
        // Check for glob patterns
        if let Dependency::Files { files } = &npm_command.depends_on[0] {
            assert_eq!(files, "package*.json");
        } else {
            panic!("Expected Files dependency");
        }
    }

    #[test]
    fn test_load_line_patterns() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/line_patterns.cacher.yaml")).unwrap();
        
        let cat_command = hint_file.commands.iter().find(|c| c.pattern == "cat config.json").unwrap();
        
        assert_eq!(cat_command.depends_on.len(), 1);
        
        if let Dependency::Lines { lines } = &cat_command.depends_on[0] {
            assert_eq!(lines.file, ".env");
            assert_eq!(lines.pattern, "^DB_*");
        } else {
            panic!("Expected Lines dependency");
        }
    }

    #[test]
    fn test_load_complex() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/complex.cacher.yaml")).unwrap();
        
        assert_eq!(hint_file.default.ttl, Some(3600));
        assert_eq!(hint_file.default.include_env.len(), 2);
        assert_eq!(hint_file.commands.len(), 2);
        
        let npm_command = hint_file.commands.iter().find(|c| c.pattern == "npm run build").unwrap();
        assert_eq!(npm_command.ttl, Some(7200));
        assert_eq!(npm_command.include_env.len(), 1);
        assert_eq!(npm_command.depends_on.len(), 4);
        
        // Check for complex glob pattern
        let src_files_dep = npm_command.depends_on.iter().find(|d| {
            if let Dependency::Files { files } = d {
                files == "src/**/*.{js,jsx,ts,tsx}"
            } else {
                false
            }
        });
        assert!(src_files_dep.is_some());
    }

    #[test]
    fn test_find_matching_command() {
        let hint_file = HintFile::from_file(Path::new("tests/fixtures/command_patterns.cacher.yaml")).unwrap();
        
        let ls_match = hint_file.find_matching_command("ls -la");
        assert!(ls_match.is_some());
        assert_eq!(ls_match.unwrap().ttl, Some(60));
        
        let git_match = hint_file.find_matching_command("git status");
        assert!(git_match.is_some());
        
        let no_match = hint_file.find_matching_command("echo hello");
        assert!(no_match.is_none());
    }
    
    #[test]
    fn test_no_hint_file() {
        // Create a temporary directory that definitely doesn't have a hint file
        let temp_dir = tempfile::tempdir().unwrap();
        let temp_path = temp_dir.path();
        
        // Try to find a hint file in the temp directory
        let hint_file = HintFile::find_hint_file(temp_path);
        
        // Should return None since there's no hint file
        assert!(hint_file.is_none());
    }
}
