export function CredentialCardSkeleton() {
  return (
    <div className="cred-card" aria-busy="true" aria-label="Loading credential">
      {/* Header skeleton */}
      <div className="cred-card__header" style={{ background: 'var(--bg-secondary)' }}>
        <div
          style={{
            height: '20px',
            width: '80px',
            background: 'var(--bg-tertiary)',
            borderRadius: '4px',
            animation: 'pulse 2s infinite',
          }}
        />
        <div
          style={{
            height: '20px',
            width: '60px',
            background: 'var(--bg-tertiary)',
            borderRadius: '4px',
            animation: 'pulse 2s infinite',
          }}
        />
      </div>

      {/* Body skeleton */}
      <div className="cred-card__body">
        <div
          style={{
            height: '18px',
            width: '120px',
            background: 'var(--bg-tertiary)',
            borderRadius: '4px',
            marginBottom: '16px',
            animation: 'pulse 2s infinite',
          }}
        />

        <div className="cred-card__meta">
          {[1, 2, 3].map((i) => (
            <div key={i} className="meta-row">
              <div
                style={{
                  height: '14px',
                  width: '60px',
                  background: 'var(--bg-tertiary)',
                  borderRadius: '4px',
                  animation: 'pulse 2s infinite',
                }}
              />
              <div
                style={{
                  height: '14px',
                  width: '100px',
                  background: 'var(--bg-tertiary)',
                  borderRadius: '4px',
                  animation: 'pulse 2s infinite',
                }}
              />
            </div>
          ))}
        </div>

        {/* Attestors skeleton */}
        <div className="cred-card__attestors" style={{ marginTop: '16px' }}>
          <div
            style={{
              height: '14px',
              width: '80px',
              background: 'var(--bg-tertiary)',
              borderRadius: '4px',
              marginBottom: '8px',
              animation: 'pulse 2s infinite',
            }}
          />
          <div
            style={{
              height: '40px',
              background: 'var(--bg-tertiary)',
              borderRadius: '4px',
              animation: 'pulse 2s infinite',
            }}
          />
        </div>
      </div>

      {/* Footer skeleton */}
      <div className="cred-card__footer">
        <div
          style={{
            height: '36px',
            background: 'var(--bg-tertiary)',
            borderRadius: '4px',
            animation: 'pulse 2s infinite',
          }}
        />
      </div>

      <style>{`
        @keyframes pulse {
          0%, 100% { opacity: 0.6; }
          50% { opacity: 1; }
        }
      `}</style>
    </div>
  );
}
