import { FeedbackForm } from '../components/FeedbackForm';
import { useWallet } from '../hooks';

export default function Feedback() {
  const { address } = useWallet();
  return (
    <div className="page-container">
      <h1 className="page-title">Share Your Feedback</h1>
      <p className="page-subtitle">Help us improve QuorumProof by sharing your experience.</p>
      <FeedbackForm walletAddress={address ?? undefined} />
    </div>
  );
}
