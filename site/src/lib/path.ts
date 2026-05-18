/** Canonical base URL with a guaranteed trailing slash. */
const raw = import.meta.env.BASE_URL
export const base = raw.endsWith('/') ? raw : raw + '/'

/** Build a site-root-relative URL. */
export const link = (path: string) => base + path.replace(/^\//, '')
