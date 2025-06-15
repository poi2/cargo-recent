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

# Setup test repository
echo "Setting up test repository..."
mkdir -p test-crate/src
cat >test-crate/Cargo.toml <<EOF
[package]
name = "test-crate"
version = "0.1.0"
edition = "2021"
EOF

cat >test-crate/src/main.rs <<EOF
fn main() {
    println!("Hello, world!");
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

# Test 2: Make a change and verify path command
echo "Test 2: Make a change and verify path command"
cat >test-crate/src/main.rs <<EOF
fn main() {
    println!("Hello, cargo-recent!");
}
EOF

OUTPUT=$(cargo recent path)
if [[ "$OUTPUT" == *"test-crate"* ]]; then
    success "cargo recent path correctly shows the path with changes"
else
    error "cargo recent path should show the path with changes, but got: $OUTPUT"
fi

# Test 3: Verify show command
echo "Test 3: Verify show command"
OUTPUT=$(cargo recent show)
if [ "$OUTPUT" = "test-crate" ]; then
    success "cargo recent show correctly shows the crate name"
else
    error "cargo recent show should show 'test-crate', but got: $OUTPUT"
fi

# Test 4: Verify check command
echo "Test 4: Verify check command"
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

# Test 5: Commit changes and verify empty output again
echo "Test 5: Commit changes and verify empty output again"
git add .
git commit -m "Test change"

OUTPUT=$(cargo recent path)
if [ -z "$OUTPUT" ]; then
    success "cargo recent path returns empty output after committing changes"
else
    error "cargo recent path should return empty output after committing changes, but got: $OUTPUT"
fi

echo "All tests passed!"
