#!/bin/bash

# Update and upgrade the system
sudo apt update -y
sudo apt upgrade -y

# Install required dependencies
sudo apt install -y curl git mercurial make binutils bison gcc build-essential

# Install Go using apt to satisfy gvm's initial requirements
sudo apt install -y golang

# Install GVM
bash < <(curl -sSL https://raw.githubusercontent.com/moovweb/gvm/master/binscripts/gvm-installer)

# Source GVM script
source ~/.gvm/scripts/gvm

# Install Go 1.20 (or any other version you need)
gvm install go1.20

# Set the default Go version
gvm use go1.20 --default

# Verify Go installation
go version

# Add GVM sourcing to .bashrc for persistence across sessions
echo "source ~/.gvm/scripts/gvm" >> ~/.bashrc

echo "Go environment setup is complete. You can switch Go versions using GVM commands."
echo "Example: gvm use go1.18"

# Optional: Clean up
sudo apt autoremove -y
sudo apt clean

# End of script
