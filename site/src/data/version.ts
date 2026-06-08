import { readFileSync } from 'node:fs'

// Single source of truth for the site's displayed version: the root Cargo.toml
// [package] version. Read at build time so the marketing site never goes stale
// after a release bump.
const cargoToml = readFileSync(new URL('../../../Cargo.toml', import.meta.url), 'utf-8')
const pkgSection = cargoToml.split(/^\[/m)[1] ?? cargoToml // [package] is the first table
const match = pkgSection.match(/^version\s*=\s*"([^"]+)"/m)

/** Cargo package version without the leading `v`, e.g. `1.5.0`. */
export const VERSION = match?.[1] ?? '0.0.0'

/** Tagged form with a leading `v`, e.g. `v1.5.0`. */
export const VERSION_TAG = `v${VERSION}`

/** Major.minor only, e.g. `v1.5`. */
export const VERSION_MINOR = `v${VERSION.split('.').slice(0, 2).join('.')}`
