import { useParams } from 'react-router-dom';

export default function CredentialDetail() {
  const { id } = useParams<{ id: string }>();

  return (
    <div>
      <h1>Credential Detail</h1>
      <p>Credential ID: {id}</p>
      {/* Placeholder for credential detail */}
    </div>
  );
}