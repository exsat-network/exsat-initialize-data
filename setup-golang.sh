#!/bin/bash

# Update and upgrade the system
sudo apt update -y
sudo apt upgrade -y

# Install required dependencies
sudo apt install -y curl git mercurial make binutils bison gcc build-essential

# Install GVM
bash < <(curl -sSL https://raw.githubusercontent.com/moovweb/gvm/master/binscripts/gvm-installer)

# Source GVM script
source ~/.gvm/scripts/gvm

# Install Go 1.19 (or any other version you need)
gvm install go1.19

# Set the default Go version
gvm use go1.19 --default


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
