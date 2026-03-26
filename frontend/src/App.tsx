import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Dashboard } from './pages/Dashboard';
import { Verify } from './pages/Verify';
import { IssueCredential } from './pages/IssueCredential';
import './styles.css';
import './index.css';

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/" element={<Navigate to="/dashboard" replace />} />
        <Route path="/dashboard" element={<Dashboard />} />
        <Route path="/verify" element={<Verify />} />
        <Route path="/issue" element={<IssueCredential />} />
      </Routes>
    </BrowserRouter>
  );
}
