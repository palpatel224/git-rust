# Git Implementation in Rust

## Features

This implementation supports core Git operations:

- `init` - Initialize a new Git repository
- `clone` - Clone a remote repository
- `cat-file` - Display Git object contents
- `hash-object` - Create Git objects from files
- `ls-tree` - List contents of a tree object
- `write-tree` - Create a tree object from the working directory
- `commit-tree` - Create a commit object

## Prerequisites

- Rust 1.80+ with Cargo

## Usage

### Quick Start

```sh
# Build and run
cargo build
# run directly with cargo
cargo run -- init
```

## Project Structure

- `src/main.rs` - Entry point and command routing
- `src/commands/` - Individual Git command implementations
- `src/objects.rs` - Git object handling (blobs, trees, commits)
- `src/error.rs` - Error handling utilities

## Learning Goals

This project demonstrates:

- Git's internal object model (blobs, trees, commits)
- Repository initialization and structure
- Object compression and storage
- Git's transfer protocols
- SHA-1 hashing and content addressing
