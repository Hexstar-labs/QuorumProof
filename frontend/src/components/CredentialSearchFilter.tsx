import { useState } from 'react';

interface SearchFilters {
  subject?: string;
  issuer?: string;
  credentialType?: number;
  startDate?: number;
  endDate?: number;
}

interface Props {
  onSearch: (filters: SearchFilters) => void;
  loading?: boolean;
}

export function CredentialSearchFilter({ onSearch, loading }: Props) {
  const [filters, setFilters] = useState<SearchFilters>({});

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    onSearch(filters);
  };

  const handleReset = () => {
    setFilters({});
    onSearch({});
  };

  return (
    <form onSubmit={handleSubmit} className="search-filter-form">
      <div className="filter-grid">
        <div className="form-group">
          <label htmlFor="subject">Subject Address</label>
          <input
            id="subject"
            type="text"
            placeholder="G..."
            value={filters.subject || ''}
            onChange={(e) => setFilters({ ...filters, subject: e.target.value || undefined })}
            className="form-input"
          />
        </div>

        <div className="form-group">
          <label htmlFor="issuer">Issuer Address</label>
          <input
            id="issuer"
            type="text"
            placeholder="G..."
            value={filters.issuer || ''}
            onChange={(e) => setFilters({ ...filters, issuer: e.target.value || undefined })}
            className="form-input"
          />
        </div>

        <div className="form-group">
          <label htmlFor="credentialType">Credential Type</label>
          <select
            id="credentialType"
            value={filters.credentialType || ''}
            onChange={(e) => setFilters({ ...filters, credentialType: e.target.value ? Number(e.target.value) : undefined })}
            className="form-input"
          >
            <option value="">All Types</option>
            <option value="1">Type 1</option>
            <option value="2">Type 2</option>
            <option value="3">Type 3</option>
            <option value="4">Type 4</option>
            <option value="5">Type 5</option>
          </select>
        </div>
      </div>

      <div className="filter-actions">
        <button type="submit" className="btn btn--primary" disabled={loading}>
          {loading ? 'Searching...' : 'Search'}
        </button>
        <button type="button" className="btn btn--ghost" onClick={handleReset} disabled={loading}>
          Reset
        </button>
      </div>
    </form>
  );
}
