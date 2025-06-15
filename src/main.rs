use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local};
use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

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

            let cargo_cmd = &args[0];
            let mut cmd = Command::new("cargo");
            cmd.arg(cargo_cmd).arg("--package").arg(crate_name);

            // Add any additional arguments
            if args.len() > 1 {
                cmd.args(&args[1..]);
            }

            let output = cmd
                .output()
                .with_context(|| format!("Failed to execute cargo {}", cargo_cmd))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow!(
                    "Command 'cargo {}' failed with error: {}",
                    cargo_cmd,
                    stderr
                ));
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
    // Get git diff to find changed files (uncommitted changes only)
    let output = Command::new("git")
        .args(["diff", "--name-only"])
        .output()
        .context("Failed to execute git diff command")?;

    if !output.status.success() {
        return Err(anyhow!("Git diff command failed"));
    }

    let diff_output =
        String::from_utf8(output.stdout).context("Failed to parse git diff output")?;

    if diff_output.trim().is_empty() {
        // Return empty path instead of error when no changes are detected
        return Ok(PathBuf::new());
    }

    // Parse the changed files and find the most recently modified one
    let mut latest_time = DateTime::<Local>::from(std::time::SystemTime::UNIX_EPOCH);
    let mut latest_file: Option<PathBuf> = None;

    for file in diff_output.lines() {
        let file_path = Path::new(file);
        if file_path.exists() {
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    let modified_time: DateTime<Local> = modified.into();
                    if modified_time > latest_time {
                        latest_time = modified_time;
                        latest_file = Some(file_path.to_path_buf());
                    } else if modified_time == latest_time && latest_file.is_some() {
                        // Tiebreak by filename (ASC sort)
                        if let Some(ref current_latest) = latest_file {
                            if file_path.to_string_lossy() < current_latest.to_string_lossy() {
                                latest_file = Some(file_path.to_path_buf());
                            }
                        }
                    }
                }
            }
        }
    }

    let latest_file = latest_file.ok_or_else(|| anyhow!("No valid changed files found"))?;

    // Find the crate directory containing this file
    find_crate_directory(&latest_file)
}

/// Find the crate directory containing the given file
fn find_crate_directory(file_path: &Path) -> Result<PathBuf> {
    // First, check if the current directory has a Cargo.toml file
    let current_dir = Path::new(".");
    let current_cargo_toml = current_dir.join("Cargo.toml");
    if current_cargo_toml.exists() {
        return Ok(current_dir.to_path_buf());
    }

    // If not, traverse up from the file's parent directory
    let mut current = file_path.parent().unwrap_or(Path::new("."));

    // Traverse up until we find a directory with a Cargo.toml file
    while let Some(parent) = current.parent() {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            return Ok(current.to_path_buf());
        }

        // Move to the parent directory
        current = parent;
    }

    // If we couldn't find a crate directory, check if we're in a workspace
    // and the file is in a subdirectory
    let workspace_cargo = Path::new("Cargo.toml");
    if workspace_cargo.exists() {
        // Check if this is a workspace by looking for [workspace] in Cargo.toml
        let cargo_content =
            fs::read_to_string(workspace_cargo).context("Failed to read workspace Cargo.toml")?;

        if cargo_content.contains("[workspace]") {
            // Look for crates in subdirectories
            for entry in WalkDir::new(".")
                .max_depth(3) // Limit depth to avoid excessive searching
                .into_iter()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file() && e.file_name() == "Cargo.toml")
            {
                let dir = entry.path().parent().unwrap_or(Path::new("."));
                let rel_path = file_path.strip_prefix(dir).ok();

                if rel_path.is_some() && !rel_path.unwrap().as_os_str().is_empty() {
                    return Ok(dir.to_path_buf());
                }
            }
        }

        // If it's not a workspace but has a Cargo.toml, use the current directory
        return Ok(Path::new(".").to_path_buf());
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
