import React, { useState } from "react";
import { AppLayout } from "./AppLayout";

/**
 * AppLayoutExample — demonstrates AppLayout wrapping a sample page.
 *
 * Usage: mount this component at your app root or a demo route.
 */
export function AppLayoutExample() {
  // Simulate route changes without a router dependency
  const [currentPath, setCurrentPath] = useState("/dashboard");

  const DEMO_WALLET = "GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCXYZ";

  return (
    <AppLayout currentPath={currentPath} walletAddress={DEMO_WALLET}>
      <div className="max-w-2xl space-y-6">
        <h1 className="text-2xl font-bold text-white">Dashboard</h1>
        <p className="text-slate-400">
          Welcome to QuorumProof. Use the navigation to verify credentials,
          manage your quorum slice, or adjust settings.
        </p>

        {/* Demo nav switcher — simulates route changes */}
        <div className="rounded-lg border border-slate-700 bg-slate-800 p-4 space-y-2">
          <p className="text-xs text-slate-500 uppercase tracking-wide">
            Demo: switch active route
          </p>
          <div className="flex flex-wrap gap-2">
            {["/dashboard", "/verify", "/slice", "/settings"].map((path) => (
              <button
                key={path}
                onClick={() => setCurrentPath(path)}
                className={`rounded px-3 py-1 text-sm font-medium transition-colors
                  ${currentPath === path
                    ? "bg-indigo-600 text-white"
                    : "bg-slate-700 text-slate-300 hover:bg-slate-600"
                  }`}
              >
                {path}
              </button>
            ))}
          </div>
          <p className="text-xs text-slate-400">
            Active: <code className="text-indigo-400">{currentPath}</code>
          </p>
        </div>

        {/* Sample credential card */}
        <div className="rounded-lg border border-slate-700 bg-slate-800 p-4 space-y-3">
          <h2 className="text-sm font-semibold text-slate-300 uppercase tracking-wide">
            Recent Credential
          </h2>
          <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
            <dt className="text-slate-500">ID</dt>
            <dd className="font-mono text-slate-200">42</dd>
            <dt className="text-slate-500">Type</dt>
            <dd className="text-slate-200">Engineering</dd>
            <dt className="text-slate-500">Status</dt>
            <dd className="text-emerald-400 font-medium">Active</dd>
            <dt className="text-slate-500">Attestors</dt>
            <dd className="text-slate-200">3 / 3</dd>
          </dl>
        </div>
      </div>
    </AppLayout>
  );
}
