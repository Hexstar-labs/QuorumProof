# Installation & Dependency Setup Guide

## Prerequisites

The dashboard requires Node.js v18+ and npm v9+ to be installed on your system.

### Check if Node.js is Installed

```bash
node --version
npm --version
```

If you see version numbers, you're good to go. If not, proceed with installation below.

## Installation by Operating System

### 🐧 Linux (Ubuntu/Debian)

```bash
# Update package manager
sudo apt update

# Install Node.js and npm
sudo apt install -y nodejs npm

# Verify installation
node --version
npm --version
```

### 🍎 macOS

Using Homebrew (recommended):

```bash
# Install Homebrew if not already installed
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install Node.js (npm comes with it)
brew install node

# Verify installation
node --version
npm --version
```

Alternatively, download from [nodejs.org](https://nodejs.org)

### 🪟 Windows

1. Download the installer from [nodejs.org](https://nodejs.org)
2. Run the installer and follow the setup wizard
3. Restart your terminal/command prompt
4. Verify installation:
   ```bash
   node --version
   npm --version
   ```

## Installation Steps

Once Node.js and npm are installed:

### 1. Navigate to Dashboard Directory

```bash
cd /home/eunice/QuorumProof/dashboard
```

### 2. Install Project Dependencies

```bash
npm install
```

This will install all required packages:
- `react` - UI library
- `react-dom` - React DOM rendering
- `lucide-react` - Icon library
- `clsx` - CSS class utility
- `date-fns` - Date formatting
- `vite` - Build tool
- `typescript` - Type safety
- And all dev dependencies

### 3. Verify Installation

```bash
npm run type-check
```

This should complete without errors if everything is installed correctly.

## Starting Development

Once dependencies are installed:

```bash
npm run dev
```

Open `http://localhost:5173` in your browser to see the CredentialCard component showcase.

## Troubleshooting

### "npm: command not found"
- Node.js/npm is not installed. Follow the installation steps above.

### "Cannot find module 'react'"
- Dependencies not installed. Run `npm install` again.

### Port 5173 Already in Use
- The dev server will automatically use the next available port.
- Or specify a different port: `npm run dev -- --port 3000`

### Build Errors
```bash
# Clear cache and reinstall
rm -rf node_modules package-lock.json
npm install
```

### TypeScript Errors
```bash
# Check for type issues
npm run type-check

# Lint code
npm run lint
```

## Available npm Scripts

After installation, you can use these commands:

```bash
npm run dev           # Start development server
npm run build         # Build for production
npm run preview       # Preview production build
npm run type-check    # Check TypeScript types
npm run lint          # Run ESLint
```

## Next Steps

1. Install Node.js if not already done
2. Run `npm install` in the dashboard directory
3. Run `npm run dev`
4. Open http://localhost:5173
5. View the CredentialCard component showcase

## Additional Resources

- [Node.js Official Site](https://nodejs.org)
- [npm Documentation](https://docs.npmjs.com)
- [Vite Documentation](https://vitejs.dev)
- [React Documentation](https://react.dev)

## Need Help?

If you encounter any issues:

1. Check this guide for your specific OS
2. Verify Node.js version: `node --version` (should be 18+)
3. Clear cache: `rm -rf node_modules package-lock.json`
4. Reinstall: `npm install`
5. Try again: `npm run dev`
