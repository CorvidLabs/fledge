import { marked } from 'marked'
import DOMPurify from 'isomorphic-dompurify'

export function renderReadme(markdown: string | null | undefined): string {
  if (!markdown) return ''
  const rawHtml = marked.parse(markdown, { async: false }) as string
  return DOMPurify.sanitize(rawHtml, {
    USE_PROFILES: { html: true },
    FORBID_TAGS: ['style'],
    FORBID_ATTR: ['style'],
  })
}
