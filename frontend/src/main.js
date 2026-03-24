/**
 * main.js — SPA router for QuorumProof Frontend
 *
 * Routes:
 *   /         → home (redirects to /verify for now)
 *   /verify   → credential verification page
 */

import './styles.css';
import { renderVerifyPage } from './verify.js';
import { renderDashboardPage } from './dashboard.js';

const app = document.getElementById('app');

function route() {
  const path = window.location.pathname;
  if (path === '/verify' || path === '/verify.html') {
    renderVerifyPage(app);
  } else if (path === '/dashboard' || path === '/dashboard.html') {
    renderDashboardPage(app);
  } else {
    // Default: redirect to /dashboard
    window.history.replaceState({}, '', '/dashboard' + window.location.search);
    renderDashboardPage(app);
  }
}

// Initial route
route();

// Handle browser navigation (back/forward)
window.addEventListener('popstate', route);

// Export router utility to easily switch pages from the navbar
export function navigateTo(path) {
  window.history.pushState({}, '', path);
  route();
}
