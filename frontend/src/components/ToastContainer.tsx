import { useToast, type Toast } from '../context/ToastContext';

const ICONS: Record<Toast['type'], string> = {
  pending: '⏳',
  success: '✅',
  error: '⚠️',
};

export function ToastContainer() {
  const { toasts, removeToast } = useToast();

  if (toasts.length === 0) return null;

  return (
    <div
      role="region"
      aria-label="Notifications"
      aria-live="polite"
      style={{
        position: 'fixed',
        bottom: '24px',
        right: '24px',
        zIndex: 9999,
        display: 'flex',
        flexDirection: 'column',
        gap: '10px',
        maxWidth: '360px',
        width: '100%',
      }}
    >
      {toasts.map((toast) => (
        <div
          key={toast.id}
          role={toast.type === 'error' ? 'alert' : 'status'}
          style={{
            display: 'flex',
            alignItems: 'flex-start',
            gap: '10px',
            padding: '12px 14px',
            borderRadius: '8px',
            background: 'var(--bg-secondary, #1e2030)',
            border: `1px solid ${
              toast.type === 'error'
                ? 'var(--color-error, #ef4444)'
                : toast.type === 'success'
                ? 'var(--color-success, #22c55e)'
                : 'var(--border, #2e303a)'
            }`,
            boxShadow: 'var(--shadow)',
            color: 'var(--text-primary, #f3f4f6)',
            fontSize: '14px',
          }}
        >
          <span aria-hidden="true" style={{ flexShrink: 0, fontSize: '16px' }}>
            {ICONS[toast.type]}
          </span>
          <div style={{ flex: 1, minWidth: 0 }}>
            <div>{toast.message}</div>
            {toast.explorerUrl && (
              <a
                href={toast.explorerUrl}
                target="_blank"
                rel="noopener noreferrer"
                style={{ color: 'var(--accent, #c084fc)', fontSize: '12px', marginTop: '4px', display: 'inline-block' }}
              >
                View on Stellar Explorer ↗
              </a>
            )}
          </div>
          <button
            onClick={() => removeToast(toast.id)}
            aria-label="Dismiss notification"
            style={{
              background: 'none',
              border: 'none',
              cursor: 'pointer',
              color: 'var(--text-secondary, #9ca3af)',
              fontSize: '16px',
              padding: '0',
              flexShrink: 0,
              lineHeight: 1,
            }}
          >
            ×
          </button>
        </div>
      ))}
    </div>
  );
}
