# QuorumProof Dashboard - Setup & Installation Guide

## Prerequisites

- Node.js 18.0.0 or higher
- npm 9.0.0 or higher (or yarn 3.0+)

## Quick Start

### 1. Install Dependencies

Navigate to the dashboard directory and install all required packages:

```bash
cd dashboard
npm install
```

**Or using Yarn:**
```bash
cd dashboard
yarn install
```

### 2. Start Development Server

```bash
npm run dev
```

The dashboard will be available at `http://localhost:5173`

### 3. Build for Production

```bash
npm run build
```

This will create an optimized build in the `dist/` directory.

## Scripts

- `npm run dev` - Start development server with hot module reloading
- `npm run build` - Build for production
- `npm run preview` - Preview production build locally
- `npm run lint` - Run ESLint to check code quality
- `npm run type-check` - Run TypeScript type checking

## Project Structure

```
dashboard/
├── src/
│   ├── components/          # React components
│   │   ├── CredentialCard.tsx    # Main credential card component
│   │   └── index.ts              # Component exports
│   ├── types/              # TypeScript type definitions
│   │   └── credential.ts   # Credential interfaces and types
│   ├── styles/             # CSS stylesheets
│   │   └── credentialCard.css
│   ├── App.tsx             # Demo/showcase application
│   ├── App.css             # App styles
│   ├── main.tsx            # React entry point
│   └── index.css           # Global styles
├── index.html              # HTML template
├── package.json            # Project dependencies
├── tsconfig.json           # TypeScript configuration
├── vite.config.ts          # Vite build configuration
├── .eslintrc.cjs           # ESLint configuration
├── .gitignore              # Git ignore rules
└── README.md               # Component documentation
```

## Troubleshooting

### Module not found errors
If you see errors like "Cannot find module 'react'":
1. Ensure you're in the `dashboard/` directory
2. Run `npm install` again
3. Clear Node cache: `npm cache clean --force`

### Build errors
If the build fails:
1. Run `npm run type-check` to identify TypeScript errors
2. Ensure all dependencies are installed
3. Try deleting `node_modules` and running `npm install` again

### Port already in use
If port 5173 is already in use:
1. The dev server will automatically try the next available port
2. Or you can specify a port: `npm run dev -- --port 3000`

## Browser Support

- Chrome/Edge 90+
- Firefox 88+
- Safari 14+
- Mobile browsers

## Development Tips

### Component Development
- The CredentialCard component supports keyboard navigation (Tab, Enter, Space)
- All components have full TypeScript support with proper type definitions
- CSS uses BEM methodology for consistency

### Styling
- Global styles are in `src/index.css`
- Component styles are co-located in `src/styles/`
- The app supports both light and dark modes via CSS media queries

### Accessibility
- All interactive elements have proper ARIA labels
- Components respect `prefers-reduced-motion` for animations
- Full keyboard navigation support throughout

## Git Workflow

This project uses feature branches for development:

```bash
# Create a feature branch
git checkout -b feature/your-feature-name

# Make your changes and commit
git add .
git commit -m "feat: description of your feature"

# Push to remote
git push -u origin feature/your-feature-name

# Create a pull request on GitHub
```

## Performance Notes

- Lucide React is used for icons (tree-shakeable SVG icons)
- date-fns is used for date formatting (only needed functions are bundled)
- React is set to production mode in build configuration
- CSS is minified in production builds

## Next Steps

1. Install dependencies: `npm install`
2. Start the dev server: `npm run dev`
3. Open `http://localhost:5173` in your browser
4. See the CredentialCard component showcase with mock data

For detailed component documentation, see [README.md](./README.md)
