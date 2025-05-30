use cacher::CommandCache;
use clap::{Parser, Subcommand};

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
    },
}

fn main() {
    let cli = Cli::parse();
    let mut cache = CommandCache::new();
    
    match &cli.command {
        Some(Commands::Run { command, args }) => {
            // Combine command and args into a single string
            let full_command = format!("{} {}", command, args.join(" ")).trim().to_string();
            
            match cache.execute_and_cache(&full_command) {
                Ok(output) => println!("{}", output),
                Err(e) => eprintln!("Error executing command: {}", e),
            }
        }
        None => {
            println!("Cacher CLI - A tool for caching command outputs");
            println!("Use --help for usage information");
        }
    }
}
