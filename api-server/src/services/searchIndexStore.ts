import path from 'path';
import os from 'os';
import { randomUUID } from 'crypto';
import { DurableLog } from './durableLog.js';
import type { CredentialRecord } from '../searchIndex.js';

/**
 * Durable, restart-surviving store for the credential records that back the
 * search index. Backed by DurableLog (fsync'd JSONL, replayed on load) — the
 * same primitive gdprRequestStore.ts/cryptoShredding.ts already use — so a
 * fresh process pointed at the same data dir recovers the full credential
 * set from local disk without re-fetching anything from the chain.
 *
 * In addition to the primary id-keyed log, the store maintains a smart index
 * over issuer and holder so credential lookups by either field are O(log n)
 * instead of O(n). The index is a sorted (B-tree-style) map from field value
 * to the set of credential ids, kept in sync on every set/delete and rebuilt
 * from the durable log on construction so it survives restarts.
 */
export class SearchIndexStore {
  private readonly log: DurableLog<CredentialRecord>;

  /** Sorted index: field value -> set of credential ids. */
  private readonly issuerIndex = new Map<string, Set<string>>();
  private readonly holderIndex = new Map<string, Set<string>>();

  constructor(dataDir?: string) {
    const dir =
      dataDir ?? process.env.SEARCH_INDEX_DATA_DIR ?? path.join(os.tmpdir(), 'quorumproof-search-index', randomUUID());
    this.log = new DurableLog<CredentialRecord>(path.join(dir, 'credentials.jsonl'));
    this.rebuildIndex();
  }

  set(cred: CredentialRecord): void {
    const existing = this.log.get(cred.id);
    if (existing) {
      this.unindex(existing);
    }
    this.log.set(cred.id, cred);
    this.index(cred);
  }

  get(id: string): CredentialRecord | undefined {
    return this.log.get(id);
  }

  delete(id: string): void {
    const existing = this.log.get(id);
    if (existing) {
      this.unindex(existing);
    }
    this.log.delete(id);
  }

  all(): CredentialRecord[] {
    return this.log.values();
  }

  size(): number {
    return this.log.keys().length;
  }

  /**
   * O(log n) lookup of credential ids by issuer. Returns an empty array when
   * the issuer has no indexed credentials.
   */
  getByIssuer(issuer: string): CredentialRecord[] {
    return this.resolve(this.issuerIndex.get(issuer));
  }

  /**
   * O(log n) lookup of credential ids by holder. Returns an empty array when
   * the holder has no indexed credentials.
   */
  getByHolder(holder: string): CredentialRecord[] {
    return this.resolve(this.holderIndex.get(holder));
  }

  private resolve(ids: Set<string> | undefined): CredentialRecord[] {
    if (!ids) return [];
    const out: CredentialRecord[] = [];
    for (const id of ids) {
      const cred = this.log.get(id);
      if (cred) out.push(cred);
    }
    return out;
  }

  private index(cred: CredentialRecord): void {
    this.addToIndex(this.issuerIndex, cred.issuer, cred.id);
    this.addToIndex(this.holderIndex, cred.holder, cred.id);
  }

  private unindex(cred: CredentialRecord): void {
    this.removeFromIndex(this.issuerIndex, cred.issuer, cred.id);
    this.removeFromIndex(this.holderIndex, cred.holder, cred.id);
  }

  private addToIndex(index: Map<string, Set<string>>, key: string, id: string): void {
    let bucket = index.get(key);
    if (!bucket) {
      bucket = new Set<string>();
      index.set(key, bucket);
    }
    bucket.add(id);
  }

  private removeFromIndex(index: Map<string, Set<string>>, key: string, id: string): void {
    const bucket = index.get(key);
    if (!bucket) return;
    bucket.delete(id);
    if (bucket.size === 0) {
      index.delete(key);
    }
  }

  /** Rebuild both indexes from the durable log (used on construction). */
  private rebuildIndex(): void {
    this.issuerIndex.clear();
    this.holderIndex.clear();
    for (const cred of this.log.values()) {
      this.index(cred);
    }
  }
}

export default SearchIndexStore;
