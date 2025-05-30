# Cacher

A command-line tool for caching command outputs to save time when running repetitive commands.

## Features

- Cache command outputs in memory and on disk
- Retrieve cached results instead of re-running commands
- Set time-to-live (TTL) for cached entries
- Force execution to bypass cache
- List all cached commands
- Clear specific or all cached entries

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/cacher.git
cd cacher

# Build the project
cargo build --release

# Optional: Install the binary
cargo install --path .
```

## Usage

### Run a command with caching

```bash
# Basic usage
cacher run "ls -la"

# With TTL (time-to-live) in seconds
cacher run "ls -la" --ttl 3600

# Force execution (ignore cache)
cacher run "ls -la" --force
```

### List cached commands

```bash
cacher list
```

### Clear cache

```bash
# Clear all cache
cacher clear --all

# Clear specific command
cacher clear --command "ls -la"
```

## How it works

Cacher uses SHA-256 hashing to generate unique identifiers for each command. When you run a command through Cacher, it:

1. Checks if the command is already cached in memory
2. If not found in memory, checks if it's cached on disk
3. If not found or if cache is expired (based on TTL), executes the command
4. Stores the result in both memory and disk cache

The cache is stored in your system's cache directory:
- macOS: `~/Library/Caches/cacher/`
- Linux: `~/.cache/cacher/`
- Windows: `C:\Users\{username}\AppData\Local\cacher\`

## Development

### Running tests

```bash
cargo test
```

### Building documentation

```bash
cargo doc --open
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
