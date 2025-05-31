use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use std::process::Command;
use serde::{Deserialize, Serialize};

/// Types of artifacts that can be cached
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ArtifactType {
    /// A directory to be cached
    #[serde(rename = "directory")]
    Directory { path: String },
    
    /// A set of files to be cached
    #[serde(rename = "files")]
    Files { paths: Vec<String> },
    
    /// A Docker image to be cached
    #[serde(rename = "docker_image")]
    DockerImage { 
        name_from: String, 
        position: usize 
    },
}

/// Handles caching and restoring of artifacts
pub struct ArtifactManager {
    base_dir: PathBuf,
}

impl ArtifactManager {
    /// Create a new ArtifactManager
    pub fn new(base_dir: PathBuf) -> Self {
        ArtifactManager { base_dir }
    }
    
    /// Get the path where artifacts for a specific cache ID are stored
    pub fn get_artifacts_path(&self, cache_id: &str) -> PathBuf {
        let artifacts_dir = self.base_dir.join(cache_id).join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap_or_else(|_| {});
        artifacts_dir
    }
    
    /// Cache a directory artifact
    pub fn cache_directory(&self, dir_path: &Path, cache_id: &str) -> io::Result<()> {
        let artifacts_dir = self.get_artifacts_path(cache_id);
        let archive_path = artifacts_dir.join("directory.tar.gz");
        
        // Ensure the directory exists
        if !dir_path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Directory not found: {}", dir_path.display())
            ));
        }
        
        // Create tar.gz of the directory
        let dir_name = dir_path.file_name().unwrap_or_default().to_string_lossy();
        let parent_dir = dir_path.parent().unwrap_or_else(|| Path::new("."));
        
        let tar_cmd = format!(
            "tar -czf {} -C {} {}", 
            archive_path.display(),
            parent_dir.display(),
            dir_name
        );
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(&tar_cmd)
            .output()?;
            
        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to create archive: {}", 
                    String::from_utf8_lossy(&output.stderr)
                )
            ));
        }
        
        Ok(())
    }
    
    /// Restore a directory artifact
    pub fn restore_directory(&self, dir_path: &Path, cache_id: &str) -> io::Result<bool> {
        let artifacts_dir = self.get_artifacts_path(cache_id);
        let archive_path = artifacts_dir.join("directory.tar.gz");
        
        if !archive_path.exists() {
            println!("Archive not found: {}", archive_path.display());
            return Ok(false);
        }
        
        // Get the parent directory where we'll extract
        let parent_dir = dir_path.parent().unwrap_or_else(|| Path::new("."));
        
        // Remove the directory if it exists to ensure clean extraction
        if dir_path.exists() {
            fs::remove_dir_all(dir_path)?;
        }
        
        // Extract directory from archive
        let extract_cmd = format!(
            "tar -xzf {} -C {}", 
            archive_path.display(),
            parent_dir.display()
        );
        
        println!("Executing extract command: {}", extract_cmd);
        
        let output = Command::new("sh")
            .arg("-c")
            .arg(&extract_cmd)
            .output()?;
            
        if !output.status.success() {
            println!("Extraction failed: {}", String::from_utf8_lossy(&output.stderr));
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Failed to extract archive: {}", 
                    String::from_utf8_lossy(&output.stderr)
                )
            ));
        }
        
        println!("Extraction successful, directory should exist at: {}", dir_path.display());
        println!("Directory exists: {}", dir_path.exists());
        
        Ok(true)
    }
    
    /// Cache an artifact based on its type
    pub fn cache_artifact(&self, artifact: &ArtifactType, cache_id: &str, base_dir: &Path) -> io::Result<()> {
        match artifact {
            ArtifactType::Directory { path } => {
                let full_path = base_dir.join(path);
                self.cache_directory(&full_path, cache_id)
            },
            // Other artifact types will be implemented later
            _ => Ok(()),
        }
    }
    
    /// Restore an artifact based on its type
    pub fn restore_artifact(&self, artifact: &ArtifactType, cache_id: &str, base_dir: &Path) -> io::Result<bool> {
        match artifact {
            ArtifactType::Directory { path } => {
                let full_path = base_dir.join(path);
                self.restore_directory(&full_path, cache_id)
            },
            // Other artifact types will be implemented later
            _ => Ok(false),
        }
    }
}
