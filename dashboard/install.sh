#!/bin/bash

# QuorumProof Dashboard Installation Script

echo "╔════════════════════════════════════════════════════════════╗"
echo "║        QuorumProof Dashboard - Setup Script               ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if Node.js is installed
if ! command -v node &> /dev/null; then
    echo "❌ Node.js is not installed. Please install Node.js 18.0.0 or higher."
    echo "   Download from: https://nodejs.org/"
    exit 1
fi

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "❌ npm is not installed. Please install npm 9.0.0 or higher."
    exit 1
fi

echo "✓ Node.js version: $(node --version)"
echo "✓ npm version: $(npm --version)"
echo ""

# Navigate to dashboard directory
cd "$(dirname "$0")" || exit 1

if [ ! -f "package.json" ]; then
    echo "❌ package.json not found. Please run this script from the dashboard directory."
    exit 1
fi

echo "Installing dependencies..."
echo ""

# Clean install
rm -rf node_modules package-lock.json

# Install dependencies
npm install

if [ $? -eq 0 ]; then
    echo ""
    echo "╔════════════════════════════════════════════════════════════╗"
    echo "║           ✓ Installation Complete!                        ║"
    echo "╚════════════════════════════════════════════════════════════╝"
    echo ""
    echo "Next steps:"
    echo ""
    echo "  1. Start development server:"
    echo "     npm run dev"
    echo ""
    echo "  2. Open http://localhost:5173 in your browser"
    echo ""
    echo "  3. See the CredentialCard component showcase with mock data"
    echo ""
    echo "For more information, see SETUP.md"
    echo ""
else
    echo ""
    echo "❌ Installation failed. Please check the error messages above."
    exit 1
fi
