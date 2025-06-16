# cargo-recent

[![Crates.io](https://img.shields.io/crates/v/cargo-recent.svg)](https://crates.io/crates/cargo-recent)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Cargo subcommand to show and operate on the most recently changed crate in workspaces and single crate projects.

```bash
# Quick example: Run tests only on the crate you're currently working on
cargo recent test
```

## Overview

`cargo-recent` simplifies your Rust development workflow by identifying the most recently changed crate and running cargo commands on it without manual specification.

This tool is especially useful when:

- You're actively developing in a large workspace with many crates
- You want to quickly run tests or checks only on the crate you're currently working on
- You want to maintain consistent workflows between workspace and single crate projects

## Requirements

- `git` - Used to detect changed files with `git diff --name-only`
- `cargo` - Used to run cargo commands on the detected crate

## Installation

From crates.io (recommended):

```bash
cargo install cargo-recent
```

Or from local repository:

```bash
cargo install --path .
```

## Usage

### Show the path of the recently changed crate

```bash
cargo recent path
```

This will output the path to the most recently changed crate. If there are no changes, it will output an empty string.

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

## Use Cases

- Quickly check, build, or test the crate you're currently working on
- Create shell aliases or scripts that automatically operate on your active crate
- Integrate with editor workflows to streamline your development process

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
