import { useState } from 'react'
import { CredentialCard } from './components/CredentialCard'
import { AttestationPanel } from './components/AttestationPanel'
import { Credential } from './types/credential'
import { Attestor } from './types/attestor'
import './App.css'

/**
 * Mock data for demonstration
 */
const mockCredentials: Credential[] = [
  {
    id: '0x1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p',
    type: 'degree',
    title: 'Bachelor of Science in Computer Science',
    subjectAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f42D74',
    issuanceDate: new Date('2023-06-15'),
    status: 'attested',
    issuer: {
      name: 'MIT',
      icon: '🎓',
    },
    expirationDate: new Date('2033-06-15'),
  },
  {
    id: '0x9f8e7d6c5b4a3f2e1d0c9b8a7f6e5d4c',
    type: 'license',
    title: 'Professional Engineer License',
    subjectAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f42D74',
    issuanceDate: new Date('2022-03-20'),
    status: 'pending',
    issuer: {
      name: 'National Society of Professional Engineers',
      icon: '⚙️',
    },
  },
  {
    id: '0x5a4b3c2d1e0f9a8b7c6d5e4f3a2b1c0d',
    type: 'employment',
    title: 'Senior Software Engineer at ACME Corp',
    subjectAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f42D74',
    issuanceDate: new Date('2021-01-10'),
    status: 'attested',
    issuer: {
      name: 'ACME Corp',
      icon: '🏢',
    },
    expirationDate: new Date('2025-01-10'),
  },
  {
    id: '0x2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7g',
    type: 'achievement',
    title: 'AWS Certified Solutions Architect',
    subjectAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f42D74',
    issuanceDate: new Date('2020-11-05'),
    status: 'revoked',
    issuer: {
      name: 'Amazon Web Services',
      icon: '☁️',
    },
    revocationReason: 'Certification expired and not renewed',
  },
  {
    id: '0x8g7f6e5d4c3b2a1f0e9d8c7b6a5f4e3d',
    type: 'degree',
    title: 'Master of Science in Software Engineering',
    subjectAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f42D74',
    issuanceDate: new Date('2023-12-01'),
    status: 'attested',
    issuer: {
      name: 'Stanford University',
      icon: '🎓',
    },
  },
]

/**
 * Mock data for attestors
 */
const mockAttestors: Attestor[] = [
  {
    address: '0x742d35Cc6634C0532925a3b844Bc9e7595f42D74',
    hasSigned: true,
    signedAt: new Date('2024-03-20'),
    name: 'Alice',
  },
  {
    address: '0xA0b86991c6218b36d9ED5f9d7C7e5f4a1A7D93b1C',
    hasSigned: false,
    name: 'Bob',
  },
  {
    address: '0xB0b86991c6218b36d9ED5f9d7C7e5f4a1A7D93b1C',
    hasSigned: false,
    name: 'Charlie',
  },
]

function App() {
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const [showAttestation, setShowAttestation] = useState(false)

  return (
    <div className="app">
      <header className="app-header">
        <h1>QuorumProof Dashboard</h1>
        <p>CredentialCard Component Showcase</p>
      </header>

      <main className="app-main">
        <section className="credentials-section">
          <h2>Your Credentials</h2>
          {selectedId && (
            <div className="selected-info">
              <p>Selected Credential ID: {selectedId}</p>
              <button onClick={() => setSelectedId(null)}>Clear Selection</button>
            </div>
          )}

          <div className="credentials-grid">
            {mockCredentials.map((credential) => (
              <div key={credential.id}>
                <CredentialCard
                  credential={credential}
                  onNavigate={(id: string) => {
                    setSelectedId(id)
                    setShowAttestation(true)
                    console.log('Navigating to credential:', id)
                  }}
                />
              </div>
            ))}
          </div>
        </section>

        {showAttestation && selectedId && (
          <section className="attestation-section">
            <button
              className="close-attestation"
              onClick={() => setShowAttestation(false)}
            >
              Close
            </button>
            <AttestationPanel
              credential={
                mockCredentials.find((c) => c.id === selectedId) || mockCredentials[0]
              }
              attestors={mockAttestors}
              threshold={{ signed: 2, required: 3 }}
              connectedWalletAddress="0x742d35Cc6634C0532925a3b844Bc9e7595f42D74"
              onAttest={async (credentialId: string) => {
                console.log('Attesting credential:', credentialId)
                // Simulate blockchain interaction
                await new Promise((resolve) => setTimeout(resolve, 2000))
              }}
            />
          </section>
        )}

        <section className="info-section">
          <h2>Component Features</h2>
          <ul>
            <li>✓ Credential type icons with color coding</li>
            <li>✓ Truncated credential IDs</li>
            <li>✓ Subject address display</li>
            <li>✓ Issuance date with relative time</li>
            <li>✓ Status badges (Attested/Pending/Revoked)</li>
            <li>✓ Keyboard navigation (Tab, Enter, Space)</li>
            <li>✓ ARIA labels for accessibility</li>
            <li>✓ Revoked credentials with muted styling</li>
            <li>✓ Dark mode support</li>
            <li>✓ Responsive design</li>
            <li>✓ Smooth interactions with reduced-motion support</li>
          </ul>
        </section>
      </main>
    </div>
  )
}

export default App
