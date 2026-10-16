# SDK Type Definitions

The QuorumProof API server publishes its request and response types as
TypeScript declarations, so client code gets autocomplete and compile-time
checks against the real API shapes (Issue #1680).

## What you get

| Export | Kind | Purpose |
|---|---|---|
| `Credential`, `Slice`, `Webhook`, `ApiKey`, … | type | Response and request body shapes (538 types in total) |
| `GetApiVerifyByIdData`, `PostApiVerifyBatchResponses`, … | type | Per-operation request, response and error types |
| `SDK_TYPES_VERSION` | `string` constant | Version of the type surface (see [Versioning](#versioning)) |

Everything except `SDK_TYPES_VERSION` is type-only, so importing the entry point
adds no runtime code.

```ts
// From inside this repository (api-server/src/types/index.ts is the source):
import type { Credential, Slice } from './types/index.js';
import { SDK_TYPES_VERSION } from './types/index.js';
```

`npm run build:types` emits declarations to `api-server/dist/types/index.d.ts`
for consumers that read the built output.

## How the types are produced

```
openapi/openapi.json            (committed spec)
        │  npm run generate:types   (@hey-api/openapi-ts, type plugin only)
        ▼
src/types/api/types.gen.ts      (generated; committed)
        │  src/types/index.ts       (hand-written public barrel)
        │  npm run build:types      (tsconfig.types.json, declarations only)
        ▼
dist/types/index.d.ts           (published declarations)
```

- `openapi-ts.types.config.ts` reads only the committed spec, so types can be
  regenerated on a fresh checkout without starting the server.
- `npm run generate:client` is a separate, full HTTP client generator. It is
  not required to get the types.
- Never edit files under `src/types/api/` by hand. Change the route and
  OpenAPI definitions, regenerate the spec, then run `npm run generate:types`.

### Regenerating

```bash
cd api-server
# after changing anything under src/openapi/ (regenerates openapi/openapi.json):
npm run openapi:generate   # currently blocked: src/middleware/validate.ts is truncated
npm run generate:types
npm run build:types
npm test -- sdk-types.test.ts
```

`generate:types` only reads the committed `openapi/openapi.json`, so it works
even while `openapi:generate` is blocked.

## Versioning

`SDK_TYPES_VERSION` (in `src/types/version.ts`) must equal both the
`package.json` version and `info.version` in `openapi/openapi.json`.
`tests/sdk-types.test.ts` enforces this, so the three cannot drift apart.

When the surface changes:

- **Patch**: documentation or comment changes only.
- **Minor**: new types or new optional fields.
- **Major**: a type or field is removed or renamed, or a required field is added.

Update the version in all three places in the same PR.
