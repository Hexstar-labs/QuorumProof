/**
 * Issue #1680 — type-only generation for the public SDK types.
 *
 * Reads the committed spec at `openapi/openapi.json` and writes TypeScript
 * types to `src/types/api/`. Unlike `openapi-ts.config.ts` (the full HTTP
 * client, which needs the spec regenerated first), this only needs the spec,
 * so it can run on a fresh checkout.
 *
 * Run `npm run generate:types` after changing `openapi/openapi.json`, then
 * `npm run build:types` to emit declarations for consumers.
 */
import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  input: './openapi/openapi.json',
  output: './src/types/api',
  plugins: ['@hey-api/typescript'],
});
