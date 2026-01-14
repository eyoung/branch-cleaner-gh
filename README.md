# Branch Cleaner

A terminal UI application for managing local git branches. It displays your branches with their GitHub PR status and allows batch deletion of merged branches.

## Features

- Lists local git branches with their GitHub PR status (Open, Merged, No PR)
- Auto-selects merged branches for deletion (safe to delete)
- Protects important branches (`main`, `master`, `develop`, `development`, and current HEAD)
- Streaming updates - PR status appears as each branch is checked
- Keyboard-driven interface

## Requirements

- Rust toolchain (for building)
- Git repository with a GitHub remote
- `GITHUB_TOKEN` environment variable for PR status lookup

### Getting a GitHub Token

1. Go to GitHub Settings > Developer settings > Personal access tokens
2. Generate a new token with `repo` scope (or `public_repo` for public repositories only)
3. Export it: `export GITHUB_TOKEN=your_token_here`

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/branch-cleaner-gh
cd branch-cleaner-gh

# Build
cargo build --release

# The binary will be at target/release/branch-cleaner-gh
```

## Usage

```bash
# Navigate to any git repository with a GitHub remote
cd your-repo

# Run with GitHub token
GITHUB_TOKEN=your_token cargo run

# Or if installed
GITHUB_TOKEN=your_token branch-cleaner-gh
```

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate branch list |
| `Space` | Toggle branch selection |
| `d` | Delete selected branches |
| `q` | Quit |

### Status Colors

- **Green (MERGED)** - PR was merged, safe to delete
- **Yellow (OPEN)** - PR is still open, use caution
- **White (No PR)** - No associated PR found

## How It Works

1. On startup, the tool reads local branches from git
2. For each branch, it queries the GitHub API to find PRs where that branch was the source
3. Branches with merged PRs are auto-selected for deletion
4. Protected branches (`main`, `master`, `develop`, `development`) and the current branch are excluded from the list

## Running Without GitHub Token

The tool will run without a token but all branches will show "No PR" status. You won't be able to see which branches have merged PRs.

## Development

```bash
# Run tests
cargo test

# Build with in-memory mock data (no GitHub API)
cargo build --features in-memory --no-default-features

# Run integration tests (requires GITHUB_TOKEN)
cargo test -- --ignored
```

## License

MIT
