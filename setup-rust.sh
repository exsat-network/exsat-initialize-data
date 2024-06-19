#!/bin/bash

# Function to install Rust environment
install_rust() {
    echo "Updating package list and installing prerequisites..."
    sudo apt update
    sudo apt install -y curl build-essential clang llvm libclang-dev  cmake

    echo "Installing Rust using rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

    echo "Sourcing the cargo environment..."
    source $HOME/.cargo/env

    echo "Rust installation complete."
}

# Function to create a new Rust project
create_rust_project() {
    read -p "Enter the project name: " PROJECT_NAME
    read -p "Enter the project directory path: " PROJECT_PATH

    echo "Creating a new Rust project..."
    cd $PROJECT_PATH
    cargo new $PROJECT_NAME && cd $PROJECT_NAME
	  

    # Add dependencies to Cargo.toml
    echo "Adding dependencies to Cargo.toml..."
    cat >> Cargo.toml <<EOL
[dependencies]
rocksdb = "0.17.0"
md5 = "0.7.0"
EOL

    echo "Creating the project directory structure..."
    mkdir src

    echo "Project $PROJECT_NAME created at $PROJECT_PATH."
    echo "You can now add your Rust code to the src/main.rs file."
}

# Main script
echo "Select an option:"
echo "1. Initialize Rust environment"
echo "2. Create a new Rust project"
read -p "Enter your choice (1 or 2): " CHOICE

case $CHOICE in
    1)
        install_rust
        ;;
    2)
        create_rust_project
        ;;
    *)
        echo "Invalid choice. Exiting."
        ;;
esac

