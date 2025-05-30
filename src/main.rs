use cacher::CommandCache;
use clap::{Parser, Subcommand};
use std::time::{Duration, SystemTime};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a command with caching
    Run {
        /// The command to run
        #[arg(required = true)]
        command: String,
        
        /// Arguments for the command
        #[arg(num_args = 0..)]
        args: Vec<String>,
        
        /// Time-to-live for cache in seconds (default: no TTL)
        #[arg(short, long)]
        ttl: Option<u64>,
        
        /// Force execution (ignore cache)
        #[arg(short, long)]
        force: bool,
    },
    
    /// List cached commands
    List,
    
    /// Clear the cache
    Clear {
        /// Clear all cached commands
        #[arg(short, long)]
        all: bool,
        
        /// Clear a specific command
        #[arg(short, long)]
        command: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let mut cache = CommandCache::new();
    
    match &cli.command {
        Some(Commands::Run { command, args, ttl, force }) => {
            // Combine command and args into a single string
            let full_command = format!("{} {}", command, args.join(" ")).trim().to_string();
            
            // Convert TTL to Duration if provided
            let ttl_duration = ttl.map(|seconds| Duration::from_secs(seconds));
            
            match cache.execute_and_cache(&full_command, ttl_duration, *force) {
                Ok(output) => println!("{}", output),
                Err(e) => eprintln!("Error executing command: {}", e),
            }
        },
        Some(Commands::List) => {
            match cache.list_cached_commands() {
                Ok(entries) => {
                    if entries.is_empty() {
                        println!("No cached commands found.");
                    } else {
                        println!("Cached commands:");
                        for (i, (command, timestamp)) in entries.iter().enumerate() {
                            let age = format_time_ago(timestamp);
                            println!("{}. {} ({})", i + 1, command, age);
                        }
                    }
                },
                Err(e) => eprintln!("Error listing cache: {}", e),
            }
        },
        Some(Commands::Clear { all, command }) => {
            if *all {
                match cache.clear_cache(None) {
                    Ok(count) => println!("Cleared {} cached commands.", count),
                    Err(e) => eprintln!("Error clearing cache: {}", e),
                }
            } else if let Some(cmd) = command {
                match cache.clear_cache(Some(cmd)) {
                    Ok(1) => println!("Cleared cache for command: {}", cmd),
                    Ok(0) => println!("No cache found for command: {}", cmd),
                    Ok(_) => unreachable!(),
                    Err(e) => eprintln!("Error clearing cache: {}", e),
                }
            } else {
                println!("Please specify --all to clear all cache or --command to clear a specific command.");
            }
        },
        None => {
            println!("Cacher CLI - A tool for caching command outputs");
            println!("Use --help for usage information");
        }
    }
}

fn format_time_ago(timestamp: &SystemTime) -> String {
    if let Ok(duration) = SystemTime::now().duration_since(*timestamp) {
        if duration.as_secs() < 60 {
            format!("{} seconds ago", duration.as_secs())
        } else if duration.as_secs() < 3600 {
            format!("{} minutes ago", duration.as_secs() / 60)
        } else if duration.as_secs() < 86400 {
            format!("{} hours ago", duration.as_secs() / 3600)
        } else {
            format!("{} days ago", duration.as_secs() / 86400)
        }
    } else {
        "unknown time".to_string()
    }
}
