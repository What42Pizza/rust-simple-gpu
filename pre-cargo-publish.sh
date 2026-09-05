#!/bin/bash

echo "======== tests ========"
cargo test
echo "======== formatting ========"
cargo fmt
echo "======== clippy ========"
cargo clippy --all-targets --all-features -- -D warnings
echo "======== doc ========"
cargo doc --no-deps --document-private-items
echo "======== package list ========"
cargo package --list
echo "======== publish dry run ========"
cargo publish --dry-run