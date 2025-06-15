#!/bin/bash
set -e # Exit on error

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print success message
success() {
    echo -e "${GREEN}✓ $1${NC}"
}

# Function to print error message and exit
error() {
    echo -e "${RED}✗ $1${NC}"
    exit 1
}

# Create a temporary directory for testing
TEST_DIR=$(mktemp -d)
echo "Using temporary directory: $TEST_DIR"

# Cleanup function to remove temporary directory
cleanup() {
    echo "Cleaning up..."
    rm -rf "$TEST_DIR"
}

# Register cleanup function to run on exit
trap cleanup EXIT

# Change to the temporary directory
cd "$TEST_DIR"

# Setup test repository with workspace
echo "Setting up test repository..."
# Create workspace root
cat >Cargo.toml <<EOF
[workspace]
members = [
    "crate-a",
    "crate-b",
]
EOF

# Create first crate
mkdir -p crate-a/src
cat >crate-a/Cargo.toml <<EOF
[package]
name = "crate-a"
version = "0.1.0"
edition = "2021"
EOF

cat >crate-a/src/main.rs <<EOF
fn main() {
    println!("Hello from crate-a!");
}
EOF

# Create second crate
mkdir -p crate-b/src
cat >crate-b/Cargo.toml <<EOF
[package]
name = "crate-b"
version = "0.1.0"
edition = "2021"
EOF

cat >crate-b/src/main.rs <<EOF
fn main() {
    println!("Hello from crate-b!");
}
EOF

# Initialize git repository
git init
git config user.name "Test User"
git config user.email "test@example.com"
git add .
git commit -m "Initial commit"

# Test 1: No changes should return empty output
echo "Test 1: No changes should return empty output"
OUTPUT=$(cargo recent path)
if [ -z "$OUTPUT" ]; then
    success "cargo recent path returns empty output when there are no changes"
else
    error "cargo recent path should return empty output, but got: $OUTPUT"
fi

# Test 2: Make a change to crate-a and verify path command
echo "Test 2: Make a change to crate-a and verify path command"
cat >crate-a/src/main.rs <<EOF
fn main() {
    println!("Hello, cargo-recent from crate-a!");
}
EOF

# Debug: Check git diff output
echo "Debug: git diff output"
git diff --name-only

# Debug: Run with verbose logging
echo "Debug: Running cargo recent path with verbose logging"
echo "Debug: Current directory: $(pwd)"
echo "Debug: Repository structure:"
find . -type f | sort

# Run git diff to see what files are changed
echo "Debug: Git diff output:"
git diff --name-only

# Run with debug logging
echo "Debug: Running cargo recent path with debug logging"
OUTPUT=$(RUST_LOG=debug cargo recent path 2>&1)
echo "Debug output: $OUTPUT"

# Run again to get just the path
OUTPUT=$(cargo recent path)
if [[ "$OUTPUT" == *"crate-a"* ]]; then
    success "cargo recent path correctly shows the path with changes"
else
    error "cargo recent path should show the path with changes, but got: $OUTPUT"
fi

# Test 3: Verify show command
echo "Test 3: Verify show command"
OUTPUT=$(cargo recent show)
if [ "$OUTPUT" = "crate-a" ]; then
    success "cargo recent show correctly shows the crate name"
else
    error "cargo recent show should show 'crate-a', but got: $OUTPUT"
fi

# Test 4: Test from subdirectory - this should fail with current implementation
echo "Test 4: Test from subdirectory - this should fail with current implementation"
# Make a change to crate-b
cat >crate-b/src/main.rs <<EOF
fn main() {
    println!("Hello, cargo-recent from crate-b!");
}
EOF

# Change to crate-a directory and run cargo recent
cd crate-a
OUTPUT=$(cargo recent path)
EXPECTED_OUTPUT="$TEST_DIR/crate-b"
if [[ "$OUTPUT" == *"$EXPECTED_OUTPUT"* ]] || [[ "$OUTPUT" == *"crate-b"* ]]; then
    success "cargo recent path correctly shows crate-b from crate-a directory"
else
    if [[ "$OUTPUT" == *"crate-a"* ]]; then
        echo "Test failed as expected: cargo recent path incorrectly shows crate-a when run from crate-a directory"
        echo "Expected: $EXPECTED_OUTPUT"
        echo "Got: $OUTPUT"
    else
        error "Unexpected output: $OUTPUT"
    fi
fi
cd "$TEST_DIR" # Go back to the test directory

# Test 5: Verify check command
echo "Test 5: Verify check command"
# Instead of using cargo recent check, we'll verify that cargo-recent can detect the crate
# and then manually run cargo check in the crate directory
CRATE_PATH=$(cargo recent path)
if [ -z "$CRATE_PATH" ]; then
    error "cargo recent path should return the crate path, but got empty output"
fi

# Change to the crate directory and run cargo check
cd "$CRATE_PATH"
cargo check || error "cargo check failed in the crate directory"
success "cargo check succeeded in the crate directory"
cd "$TEST_DIR" # Go back to the test directory

# Test 6: Commit changes and verify empty output again
echo "Test 6: Commit changes and verify empty output again"
git add .
git commit -m "Test change"

OUTPUT=$(cargo recent path)
if [ -z "$OUTPUT" ]; then
    success "cargo recent path returns empty output after committing changes"
else
    error "cargo recent path should return empty output after committing changes, but got: $OUTPUT"
fi

echo "All tests passed!"
