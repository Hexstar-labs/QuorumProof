# Linting Standards

This page is the single reference for how code is linted in QuorumProof
(Issue #1679). It covers Rust, TypeScript/JavaScript, Python and the
whitespace rules shared by all of them.

## Summary

| Language | Linter / formatter | Config file | Where it runs |
|---|---|---|---|
| Rust | `cargo clippy` | `clippy.toml` (repo root) | `scripts/lint.sh rust` |
| TypeScript (frontend) | ESLint 9 flat config | `frontend/eslint.config.js` | `npm run lint` in `frontend/` (CI) |
| TypeScript (dashboard) | ESLint 8 legacy config | `dashboard/.eslintrc.cjs` | `npm run lint` in `dashboard/` (CI) |
| Python | `ruff check` and `ruff format` | `ruff.toml` (repo root) | `scripts/lint.sh python` |
| All files | EditorConfig | `.editorconfig` (repo root) | Editor plugin / `editorconfig-checker` |

Run everything that is installed locally with:

```bash
./scripts/lint.sh          # all linters
./scripts/lint.sh rust     # clippy only
./scripts/lint.sh python   # ruff only
./scripts/lint.sh js       # frontend + dashboard ESLint
```

## Rust

- Config: `clippy.toml` sets complexity thresholds and allows
  `unwrap`/`expect`/`dbg!`/`println!` inside tests only.
- Production contract code must not contain bare `.unwrap()` calls. That
  rule is enforced as a ratchet by `scripts/check_unwraps.sh` (#1391), not by
  clippy, so it cannot regress silently.
- Clippy is run with `-D warnings` by `scripts/lint.sh`. Fix the lint or add a
  narrowly scoped `#[allow(clippy::...)]` with a comment explaining why.

## TypeScript and JavaScript

- `frontend/` uses ESLint 9 flat config with `typescript-eslint`,
  `eslint-plugin-react-hooks` and `eslint-plugin-react-refresh`.
- `dashboard/` uses the legacy `.eslintrc.cjs` format. It is pinned to ESLint 8
  and has its own `lint:no-contract-js` check, so do not upgrade it piecemeal.
- `api-server/` has no ESLint config yet. It is type-checked with `tsc` and
  tested with Vitest. Adding ESLint there is a follow-up.
- ESLint errors fail the frontend and dashboard `npm run lint` CI steps.
  ESLint warnings do not, so treat warnings as work to clean up. Do not add
  `eslint-disable` comments without a reason.

## Python

- Config: `ruff.toml`. The rule set is deliberately small (`E`, `W`, `F`, `I`).
  Add rule families only in a PR that also fixes the findings they produce.
  Python is not yet gated in CI; it is checked by `scripts/lint.sh python`.
- Line length is 100. Formatting follows `ruff format` (double quotes,
  spaces).

## Whitespace and file hygiene

`.editorconfig` sets the defaults that apply to every file type:

- UTF-8, LF line endings, a final newline, no trailing whitespace.
- 4 spaces for Rust, Python and shell; 2 spaces for TypeScript and JSON.
- Markdown keeps trailing whitespace because some docs use it for hard line
  breaks.

Most editors read this file automatically. The VS Code defaults in
`.vscode/settings.json` match it.

## Adding or changing a linter

1. Put the config at the repo root (or next to the package it applies to).
2. Add the command to `scripts/lint.sh` so it runs locally.
3. If the rule is blocking, add it to `.github/workflows/ci.yml` in the same
   PR, and make sure the existing code passes first.
4. Update the table at the top of this page.
