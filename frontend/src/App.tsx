import React, { Suspense, lazy } from 'react';
import { BrowserRouter, Routes, Route, useLocation, Navigate } from 'react-router-dom';
import { AppLayout } from './components/AppLayout';
import { WalletGuard } from './components/WalletGuard';
import { useWallet } from './hooks';
import './styles.css';
import './index.css';

// Lazy load pages
const Dashboard = lazy(() => import('./pages/Dashboard').then(module => ({ default: module.default })));
const Verify = lazy(() => import('./pages/Verify').then(module => ({ default: module.default })));
const QuorumSlice = lazy(() => import('./pages/QuorumSlice').then(module => ({ default: module.default })));
const CredentialDetail = lazy(() => import('./pages/CredentialDetail').then(module => ({ default: module.default })));
const IssueCredential = lazy(() => import('./pages/IssueCredential').then(module => ({ default: module.default })));
const Profile = lazy(() => import('./pages/Profile').then(module => ({ default: module.default })));
const CredentialCompare = lazy(() => import('./pages/CredentialCompare').then(module => ({ default: module.default })));

// Loading fallback
const LoadingFallback = () => (
  <div className="flex items-center justify-center h-full">
    <div className="text-slate-400">Loading...</div>
  </div>
);

// 404 component
const NotFound = () => (
  <div className="flex flex-col items-center justify-center h-full">
    <h1 className="text-2xl font-bold text-slate-100 mb-4">Page not found</h1>
    <p className="text-slate-400 mb-4">The page you're looking for doesn't exist.</p>
    <a href="/" className="text-indigo-400 hover:text-indigo-300">Go back to home</a>
  </div>
);

function AppContent() {
  const location = useLocation();
  const { address, connect, network } = useWallet();

  return (
    <AppLayout
      currentPath={location.pathname}
      walletAddress={address}
      onConnectWallet={connect}
      network={network}
    >
      <Suspense fallback={<LoadingFallback />}>
        <Routes>
          <Route path="/" element={<Navigate to="/dashboard" replace />} />
          <Route path="/dashboard" element={<WalletGuard><Dashboard /></WalletGuard>} />
          <Route path="/verify" element={<Verify />} />
          <Route path="/slice/new" element={<WalletGuard><QuorumSlice /></WalletGuard>} />
          <Route path="/credential/issue" element={<WalletGuard><IssueCredential /></WalletGuard>} />
          <Route path="/credential/:id" element={<CredentialDetail />} />
          <Route path="/profile" element={<WalletGuard><Profile /></WalletGuard>} />
          <Route path="/compare" element={<CredentialCompare />} />
          <Route path="*" element={<NotFound />} />
        </Routes>
      </Suspense>
    </AppLayout>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <AppContent />
    </BrowserRouter>
  );
}
