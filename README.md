# cargo-recent

A Cargo subcommand to show and operate on recently changed crates in a workspace.

## Overview

`cargo-recent` is a tool designed to simplify working with multi-crate Rust workspaces. It identifies the most recently modified crate based on git diff and allows you to run cargo commands specifically on that crate without having to manually specify the crate name or path.

## Requirements

- `git` - Used to detect changed files with `git diff --name-only`
- `cargo` - Used to run cargo commands on the detected crate

## Installation

```bash
cargo install --path .
```

Or install from crates.io:

```bash
cargo install cargo-recent
```

## Usage

### Show the path of the recently changed crate

```bash
cargo recent path
```

This will output the path to the most recently changed crate. If there are no changes (git diff is empty), it will output an empty string.

### Show the name of the recently changed crate

```bash
cargo recent show
```

This will output the name of the most recently changed crate. If there are no changes, it will output an empty string.

### Run a cargo command on the recently changed crate

```bash
cargo recent <cargo-command> [args...]
```

For example:

```bash
cargo recent check
cargo recent build --release
cargo recent test -- --nocapture
```

This will run the specified cargo command on the most recently changed crate. If there are no changes, it will output an empty string and exit without running any cargo command.

## How It Works

1. `cargo-recent` uses `git diff --name-only` to detect changed files
2. It finds the most recently modified file among the changed files
3. It locates the crate directory containing that file
4. It extracts the crate name from the Cargo.toml file
5. It performs the requested operation on that crate

## Use Cases

- Quickly check, build, or test the crate you're currently working on in a multi-crate workspace
- Create shell aliases or scripts that automatically operate on your active crate
- Integrate with editor workflows to streamline your development process

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
