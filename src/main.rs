use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// Macro for outputting debug logs
// Only compiled when the dbg feature is enabled
#[cfg(feature = "dbg")]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        eprintln!("Debug: {}", format!($($arg)*));
    };
}

// Does nothing when the dbg feature is disabled
#[cfg(not(feature = "dbg"))]
macro_rules! debug_log {
    ($($arg:tt)*) => {
        // Do nothing
    };
}

#[derive(Parser)]
#[command(name = "cargo-recent")]
#[command(bin_name = "cargo")]
#[command(about = "A tool to show and operate on recently changed crates")]
enum Cli {
    #[command(name = "recent")]
    Recent(Args),
}

#[derive(Parser)]
#[command(about = "A tool to show and operate on recently changed crates")]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show the path of the recently changed crate
    Path,
    /// Show the name of the recently changed crate
    Show,
    /// Run a cargo command on the recently changed crate
    #[command(external_subcommand)]
    External(Vec<String>),
}

fn main() -> Result<()> {
    // Print debug information about the environment
    debug_log!("Current directory: {:?}", std::env::current_dir()?);
    debug_log!("Args: {:?}", std::env::args().collect::<Vec<_>>());

    let Cli::Recent(args) = Cli::parse();

    match args.command {
        Some(Commands::Path) => {
            let crate_path = find_recent_crate_path()?;
            if crate_path.as_os_str().is_empty() {
                // Print empty string when no changes are detected
                println!();
            } else {
                println!("{}", crate_path.display());
            }
        }
        Some(Commands::Show) => {
            let crate_path = find_recent_crate_path()?;
            if crate_path.as_os_str().is_empty() {
                // Print empty string when no changes are detected
                println!();
                return Ok(());
            }
            let crate_name = get_crate_name(&crate_path)?;
            println!("{}", crate_name);
        }
        Some(Commands::External(args)) => {
            if args.is_empty() {
                return Err(anyhow!("No cargo command specified"));
            }

            let crate_path = find_recent_crate_path()?;
            if crate_path.as_os_str().is_empty() {
                // Print empty string and exit when no changes are detected
                println!();
                return Ok(());
            }
            let crate_name = get_crate_name(&crate_path)?;

            // Create the command
            let mut cmd = Command::new("cargo");

            // Add all subcommands first
            for arg in &args {
                cmd.arg(arg);
            }

            // Then add the package flag
            cmd.arg("--package").arg(&crate_name);

            // Print the command being executed
            let mut command_str = "run: cargo".to_string();

            // Add all subcommands first
            for arg in &args {
                command_str.push(' ');
                command_str.push_str(arg);
            }

            // Then add the package flag
            command_str.push_str(" --package ");
            command_str.push_str(&crate_name);

            println!("{}", command_str);

            // Set stdout and stderr to inherit from the parent process
            // This preserves color output and other terminal features
            use std::process::Stdio;
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());

            // Execute the command and wait for it to complete
            let status = cmd
                .status()
                .with_context(|| "Failed to execute command".to_string())?;

            if !status.success() {
                return Err(anyhow!("Command failed"));
            }
        }
        None => {
            println!("No command specified. Try 'cargo recent path' or 'cargo recent show'");
        }
    }

    Ok(())
}

/// Find the path of the recently changed crate
fn find_recent_crate_path() -> Result<PathBuf> {
    debug_log!("Entering find_recent_crate_path");

    // First, try to find the repository root
    let repo_root =
        find_repo_root().ok_or_else(|| anyhow!("Could not find git repository root"))?;

    debug_log!("Repository root: {}", repo_root.display());

    // Get git diff to find changed files (uncommitted changes only)
    let output = Command::new("git")
        .args(["diff", "--name-only"])
        .current_dir(&repo_root) // Ensure we run git diff from the repository root
        .output()
        .context("Failed to execute git diff command")?;

    if !output.status.success() {
        debug_log!("Git diff command failed with status: {:?}", output.status);
        debug_log!(
            "Git diff stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return Err(anyhow!("Git diff command failed"));
    }

    let diff_output =
        String::from_utf8(output.stdout).context("Failed to parse git diff output")?;

    debug_log!("Git diff output: {:?}", diff_output);

    if diff_output.trim().is_empty() {
        debug_log!("No changes detected");
        // Return empty path instead of error when no changes are detected
        return Ok(PathBuf::new());
    }

    // Parse the changed files and find the most recently modified one
    let mut latest_time = DateTime::<Local>::from(std::time::SystemTime::UNIX_EPOCH);
    let mut latest_file: Option<PathBuf> = None;

    for file in diff_output.lines() {
        debug_log!("Processing file from git diff: {}", file);

        // Check if the file is a Rust file (.rs) or Cargo file (Cargo.toml, Cargo.lock)
        let is_rust_file = file.ends_with(".rs");
        let is_cargo_file = file.ends_with("Cargo.toml") || file.ends_with("Cargo.lock");

        if !is_rust_file && !is_cargo_file {
            debug_log!("Skipping non-Rust/Cargo file: {}", file);
            continue;
        }

        // Convert the relative path from git diff to an absolute path
        let file_path = repo_root.join(file);
        debug_log!("Absolute file path: {}", file_path.display());

        if file_path.exists() {
            debug_log!("File exists");
            if let Ok(metadata) = fs::metadata(&file_path) {
                debug_log!("Got metadata for file");
                if let Ok(modified) = metadata.modified() {
                    let modified_time: DateTime<Local> = modified.into();
                    debug_log!("File modified time: {}", modified_time);
                    if modified_time > latest_time {
                        debug_log!("New latest file: {}", file_path.display());
                        latest_time = modified_time;
                        latest_file = Some(file_path);
                    } else if modified_time == latest_time && latest_file.is_some() {
                        // Tiebreak by filename (ASC sort)
                        if let Some(ref current_latest) = latest_file {
                            if file_path.to_string_lossy() < current_latest.to_string_lossy() {
                                latest_file = Some(file_path);
                            }
                        }
                    }
                }
            }
        }
    }

    let latest_file = match latest_file {
        Some(file) => file,
        None => {
            debug_log!("No valid changed files found");
            return Err(anyhow!("No valid changed files found"));
        }
    };

    debug_log!("Latest file: {}", latest_file.display());

    // Find the crate directory containing this file
    let crate_dir = find_crate_directory(&latest_file)?;
    debug_log!("Crate directory: {}", crate_dir.display());

    Ok(crate_dir)
}

/// Find the Git repository root directory
fn find_repo_root() -> Option<PathBuf> {
    let mut current = std::env::current_dir().ok()?;

    // Traverse up until we find a .git directory
    loop {
        let git_dir = current.join(".git");
        if git_dir.exists() && git_dir.is_dir() {
            return Some(current);
        }

        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            // Reached the root of the filesystem without finding .git
            return None;
        }
    }
}

/// Find the crate directory containing the given file
fn find_crate_directory(file_path: &Path) -> Result<PathBuf> {
    debug_log!("Finding crate directory for file: {}", file_path.display());

    // Get the absolute path of the file
    let abs_file_path = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(file_path)
    };
    debug_log!("Absolute file path: {}", abs_file_path.display());

    // Start from the file's directory and traverse up until we find a Cargo.toml
    let mut current = abs_file_path.parent().unwrap_or(Path::new("/"));
    debug_log!("Starting search from directory: {}", current.display());

    // Keep track of the repository root if we find it
    let mut repo_root: Option<PathBuf> = None;

    // Traverse up until we find a directory with a Cargo.toml file or reach the filesystem root
    while current != Path::new("") && current != Path::new("/") {
        debug_log!("Checking directory: {}", current.display());

        // Check if this directory has a Cargo.toml
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            debug_log!("Found Cargo.toml at: {}", cargo_toml.display());

            // Check if this is a workspace root
            let cargo_content =
                fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;

            let is_workspace = cargo_content.contains("[workspace]");

            if is_workspace {
                debug_log!("This is a workspace root");
                // Remember this as the repository root, but continue searching
                // for a more specific crate directory
                repo_root = Some(current.to_path_buf());
            } else {
                // This is a regular crate, not a workspace root
                // Return this directory immediately
                debug_log!("Found regular crate directory: {}", current.display());
                return Ok(current.to_path_buf());
            }
        }

        // Check if this directory has a .git directory (repository root)
        if repo_root.is_none() {
            let git_dir = current.join(".git");
            if git_dir.exists() && git_dir.is_dir() {
                debug_log!("Found repository root at: {}", current.display());
                repo_root = Some(current.to_path_buf());
            }
        }

        // Move to the parent directory
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            break;
        }
    }

    // If we found a repository root with a workspace, try to find the specific crate
    // that contains the file
    if let Some(root) = repo_root {
        let root_cargo_toml = root.join("Cargo.toml");
        if root_cargo_toml.exists() {
            let cargo_content =
                fs::read_to_string(&root_cargo_toml).context("Failed to read root Cargo.toml")?;

            if cargo_content.contains("[workspace]") {
                debug_log!(
                    "Checking workspace members for file: {}",
                    abs_file_path.display()
                );

                // Try to find the most specific crate directory that contains the file
                // by traversing up from the file's directory
                let mut current = abs_file_path.parent().unwrap_or(Path::new("/"));

                while current != Path::new("") && current != Path::new("/") && current != root {
                    let cargo_toml = current.join("Cargo.toml");
                    if cargo_toml.exists() {
                        debug_log!("Found subcrate Cargo.toml at: {}", cargo_toml.display());
                        return Ok(current.to_path_buf());
                    }

                    if let Some(parent) = current.parent() {
                        current = parent;
                    } else {
                        break;
                    }
                }

                // If we couldn't find a specific crate by traversing up,
                // return the workspace root as a fallback
                debug_log!(
                    "No specific crate found, returning workspace root: {}",
                    root.display()
                );
                return Ok(root);
            }
        }

        // If it's not a workspace but has a repository root, return the root
        return Ok(root);
    }

    // If we couldn't find any Cargo.toml or repository root,
    // check if the current directory has a Cargo.toml
    let current_dir = std::env::current_dir()?;
    let current_cargo_toml = current_dir.join("Cargo.toml");
    if current_cargo_toml.exists() {
        debug_log!(
            "Using current directory as crate directory: {}",
            current_dir.display()
        );
        return Ok(current_dir);
    }

    Err(anyhow!(
        "Could not find a crate directory for the changed file"
    ))
}

/// Get the crate name from the crate directory
fn get_crate_name(crate_dir: &Path) -> Result<String> {
    let cargo_toml = crate_dir.join("Cargo.toml");
    let content = fs::read_to_string(cargo_toml).context("Failed to read Cargo.toml")?;

    // Extract the package name using regex
    let re = Regex::new(r#"(?m)^\s*name\s*=\s*"([^"]+)""#).context("Failed to compile regex")?;

    if let Some(captures) = re.captures(&content) {
        if let Some(name) = captures.get(1) {
            return Ok(name.as_str().to_string());
        }
    }

    // Fallback: use directory name
    Ok(crate_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_get_crate_name_from_cargo_toml() {
        // Create a temporary directory
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Create a Cargo.toml file with a test crate name
        let cargo_toml_path = temp_path.join("Cargo.toml");
        let mut cargo_toml = File::create(&cargo_toml_path).unwrap();
        writeln!(
            cargo_toml,
            r#"[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"
"#
        )
        .unwrap();

        // Test get_crate_name function
        let crate_name = get_crate_name(temp_path).unwrap();
        assert_eq!(crate_name, "test-crate");
    }

    #[test]
    fn test_get_crate_name_fallback() {
        // Create a temporary directory with a name but no Cargo.toml
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path().join("fallback-crate");
        fs::create_dir(&temp_path).unwrap();

        // Create an empty file that is not a valid Cargo.toml
        let invalid_cargo_toml = temp_path.join("Cargo.toml");
        File::create(&invalid_cargo_toml).unwrap();

        // This should fall back to the directory name
        let result = get_crate_name(&temp_path);
        assert!(result.is_err() || result.unwrap() == "fallback-crate");
    }

    #[test]
    fn test_find_crate_directory() {
        // Save current directory
        let original_dir = env::current_dir().unwrap();

        // Create a temporary directory structure
        let temp_dir = tempdir().unwrap();
        let temp_path = temp_dir.path();

        // Change to the temporary directory to avoid finding Cargo.toml in the current directory
        env::set_current_dir(temp_path).unwrap();

        // Create a test crate structure
        let crate_dir = temp_path.join("test-crate");
        let src_dir = crate_dir.join("src");
        let file_path = src_dir.join("main.rs");

        fs::create_dir_all(&src_dir).unwrap();
        File::create(&file_path).unwrap();

        // Create a Cargo.toml in the crate directory
        let cargo_toml_path = crate_dir.join("Cargo.toml");
        let mut cargo_toml = File::create(&cargo_toml_path).unwrap();
        writeln!(
            cargo_toml,
            r#"[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"
"#
        )
        .unwrap();

        // Test finding the crate directory from a file
        let found_dir = find_crate_directory(&file_path).unwrap();

        // Convert paths to canonical form for comparison
        let found_path = found_dir.canonicalize().unwrap();
        let expected_path = crate_dir.canonicalize().unwrap();

        assert_eq!(
            found_path,
            expected_path,
            "Found path '{}' should equal expected path '{}'",
            found_path.display(),
            expected_path.display()
        );

        // Restore original directory
        env::set_current_dir(original_dir).unwrap();
    }

    // This test requires a git repository, so we'll make it conditional
    #[test]
    #[ignore = "Requires a git repository with changes"]
    fn test_find_recent_crate_path() {
        // Save the current directory
        let _current_dir = env::current_dir().unwrap();

        // This test assumes it's run from a git repository with changes
        let crate_path = find_recent_crate_path().unwrap();

        // If there are no changes, this should return an empty path
        if crate_path.as_os_str().is_empty() {
            println!("No changes detected in git repository");
        } else {
            println!("Found crate path: {}", crate_path.display());
            // Verify that the path exists and contains a Cargo.toml
            assert!(crate_path.join("Cargo.toml").exists());
        }
    }
}
