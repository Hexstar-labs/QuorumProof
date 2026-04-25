import React from 'react';

interface ErrorBoundaryState {
  hasError: boolean;
  error?: Error;
}

export class ErrorBoundary extends React.Component<React.PropsWithChildren<{}>, ErrorBoundaryState> {
  constructor(props: React.PropsWithChildren<{}>) {
    super(props);
    this.state = { hasError: false };
  }

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('Uncaught error in ErrorBoundary:', error, info);
  }

  handleReload = () => {
    window.location.reload();
  };

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex h-screen w-full items-center justify-center bg-slate-900 px-4 text-slate-100">
          <div className="max-w-md rounded-xl border border-slate-700 bg-slate-800 p-6 text-center">
            <h1 className="text-2xl font-bold text-white">Something went wrong</h1>
            <p className="mt-2 text-slate-300">An unexpected error occurred. Please reload to continue.</p>
            <button
              className="mt-5 rounded bg-indigo-600 px-4 py-2 font-medium text-white hover:bg-indigo-500"
              onClick={this.handleReload}
            >
              Reload
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}
