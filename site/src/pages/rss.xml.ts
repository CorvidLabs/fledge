/**
 * Retired RSS feed.
 *
 * The standalone fledge blog has moved to the CorvidLabs hub. This endpoint
 * still builds at /fledge/rss.xml but now serves an HTML redirect to the hub
 * so any subscribed reader / crawler is sent to the canonical location.
 */
import { HUB_MARKETING } from '../data/hub'

export function GET() {
  const to = HUB_MARKETING
  const body = `<!doctype html>
<html lang="en"><head>
<meta charset="utf-8">
<title>Moved to CorvidLabs</title>
<link rel="canonical" href="${to}">
<meta http-equiv="refresh" content="0; url=${to}">
<meta name="robots" content="noindex">
</head><body>
<p>This feed has moved. <a href="${to}">This site has moved to CorvidLabs &rarr;</a></p>
</body></html>
`
  return new Response(body, {
    headers: { 'Content-Type': 'text/html; charset=utf-8' },
  })
}
