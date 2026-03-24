#!/bin/bash

# QuorumProof Dashboard - Automated Setup Script
# This script will install Node.js and all project dependencies

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║     QuorumProof Dashboard - Automated Setup                ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check OS
OS_TYPE=$(uname -s)
echo "Detected OS: $OS_TYPE"
echo ""

# Install Node.js and npm based on OS
if [ "$OS_TYPE" = "Linux" ]; then
    echo "Installing Node.js and npm for Linux..."
    echo ""
    echo "Run the following commands:"
    echo ""
    echo "  # Update package manager"
    echo "  sudo apt update"
    echo ""
    echo "  # Install Node.js and npm"
    echo "  sudo apt install -y nodejs npm"
    echo ""
    echo "  # Verify installation"
    echo "  node --version"
    echo "  npm --version"
    echo ""
    echo "After installation, run this script again."
    
elif [ "$OS_TYPE" = "Darwin" ]; then
    echo "Installing Node.js and npm for macOS..."
    if ! command -v brew &> /dev/null; then
        echo "Homebrew not found. Installing Homebrew first..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi
    brew install node
    
elif [ "$OS_TYPE" = "MINGW64_NT" ] || [ "$OS_TYPE" = "MSYS_NT" ]; then
    echo "Windows detected. Please download and install Node.js from:"
    echo "https://nodejs.org/en/download/"
    exit 1
fi

echo ""
echo "Installation instructions provided above."
