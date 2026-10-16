/**
 * Public type entry point for SDK consumers (Issue #1680).
 *
 * Every export here is type-only except `SDK_TYPES_VERSION`, so importing this
 * module adds no runtime code. Types are generated from the committed spec
 * `openapi/openapi.json` by `npm run generate:types`; do not edit `./api/` by
 * hand. See docs/sdk-types.md.
 */
export type * from './api/types.gen.js';
export { SDK_TYPES_VERSION } from './version.js';
