#!/bin/bash

# Navigate to the rust-client directory
cd rust-client

# Build the rust-client binary
cargo build

# Navigate back to the root directory
cd ..

# Navigate to the rust-server directory
cd rust-server

# Build the rust-server binary
cargo build

# Navigate back to the root directory
cd ..
