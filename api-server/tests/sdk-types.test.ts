// Issue #1680 — the public SDK type surface must stay in step with the package
// version and the committed OpenAPI spec it was generated from.
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';
import { SDK_TYPES_VERSION } from '../src/types/index.js';

const here = dirname(fileURLToPath(import.meta.url));
const readJson = (rel: string) =>
  JSON.parse(readFileSync(join(here, '..', rel), 'utf8'));

describe('SDK type exports (#1680)', () => {
  it('SDK_TYPES_VERSION matches package.json version', () => {
    expect(SDK_TYPES_VERSION).toBe(readJson('package.json').version);
  });

  it('SDK_TYPES_VERSION matches the OpenAPI spec info.version', () => {
    expect(SDK_TYPES_VERSION).toBe(readJson('openapi/openapi.json').info.version);
  });

  it('generated type file is present so the barrel can re-export it', () => {
    const typesFile = readFileSync(join(here, '..', 'src/types/api/types.gen.ts'), 'utf8');
    expect(typesFile).toContain('export type Credential');
  });
});
