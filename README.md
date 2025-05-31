# Cacher

A command-line tool for caching command outputs to save time when running repetitive commands.

## Features

- Cache command outputs in memory and on disk
- Retrieve cached results instead of re-running commands
- Set time-to-live (TTL) for cached entries
- Force execution to bypass cache
- List all cached commands
- Clear specific or all cached entries
- Get hash ID for any command

## Installation

### Pre-built Binaries

You can download pre-built binaries for your platform from the [latest GitHub release](https://github.com/deanshub/cacher/releases/latest):

- [Linux (x86_64)](https://github.com/deanshub/cacher/releases/latest/download/cacher-linux-amd64)
- [Linux (ARM64)](https://github.com/deanshub/cacher/releases/latest/download/cacher-linux-arm64)
- [macOS (x86_64)](https://github.com/deanshub/cacher/releases/latest/download/cacher-macos-amd64)
- [macOS (ARM64/Apple Silicon)](https://github.com/deanshub/cacher/releases/latest/download/cacher-macos-arm64)
- [Windows (x86_64)](https://github.com/deanshub/cacher/releases/latest/download/cacher-windows-amd64.exe)

After downloading, make the binary executable (Linux/macOS):

```bash
chmod +x cacher-*
```

### From Source

```bash
# Clone the repository
git clone https://github.com/deanshub/cacher.git
cd cacher

# Build the project
cargo build --release

# Optional: Install the binary
cargo install --path .
```

### Using Cargo

```bash
cargo install cacher
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

### Get hash ID for a command

```bash
cacher hash "ls -la"
```

### Using a .cacher hint file

You can create a `.cacher.yaml` file in your project to customize caching behavior. Cacher will automatically look for this file in the current directory and its parent directories.

#### Basic Configuration

```yaml
# Default settings for all commands
default:
  ttl: 3600  # Default TTL in seconds (1 hour)
  include_env:
    - PATH
    - NODE_ENV  # Include environment variables in cache key
```

#### Command Patterns

Use glob patterns to match commands:

```yaml
commands:
  - pattern: "npm run *"  # Matches all npm run commands
    ttl: 7200  # 2 hours
  
  - pattern: "git status"  # Exact match
    ttl: 60  # 1 minute
```

#### File Dependencies

Specify files that should invalidate the cache when modified:

```yaml
commands:
  - pattern: "npm run build"
    depends_on:
      - file: "package.json"  # Single file
      - files: "src/**/*.js"  # Glob pattern for multiple files
```

#### Environment Variables

Include specific environment variables in the cache key:

```yaml
commands:
  - pattern: "docker-compose up"
    include_env:
      - DOCKER_HOST
      - COMPOSE_PROJECT_NAME
```

#### Line Patterns

Only consider specific lines in files using regex patterns:

```yaml
commands:
  - pattern: "npm run dev"
    depends_on:
      - lines:
          file: ".env"
          pattern: "^(API_|DEV_)"  # Only match lines starting with API_ or DEV_
```

#### Complete Example

```yaml
# Default settings for all commands
default:
  ttl: 3600  # Default TTL in seconds
  include_env:
    - PATH
    - NODE_ENV

# Command-specific settings
commands:
  # Cache npm build commands for 2 hours
  - pattern: "npm run build"
    ttl: 7200
    include_env:
      - NODE_ENV
    depends_on:
      - files: "src/**/*.{js,jsx,ts,tsx}"  # All source files
      - files: "package*.json"             # package.json and package-lock.json
      - file: "tsconfig.json"              # Specific file
      
  # Cache docker-compose commands for 1 day
  - pattern: "docker-compose up *"
    ttl: 86400
    include_env:
      - DOCKER_HOST
    depends_on:
      - file: "docker-compose.yml"
      - files: "Dockerfile*"
      - lines:
          file: ".env"
          pattern: "^(DB_|API_)"  # Only consider DB_ and API_ variables
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
