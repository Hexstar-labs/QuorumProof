import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import { WalletProvider } from './context/WalletContext.tsx'
import { ToastProvider } from './context/ToastContext.tsx'
import { ToastContainer } from './components/ToastContainer.tsx'
import { ErrorBoundary } from './components/ErrorBoundary.tsx'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ErrorBoundary>
      <WalletProvider>
        <ToastProvider>
          <App />
          <ToastContainer />
        </ToastProvider>
      </WalletProvider>
    </ErrorBoundary>
  </StrictMode>,
)
