const CACHE = 'maritime-tiles-v1'

const TILE_ORIGINS = [
  'basemaps.cartocdn.com',
  'tiles.openseamap.org',
]

function isTileRequest(url) {
  return TILE_ORIGINS.some(o => url.hostname.includes(o))
}

self.addEventListener('install', () => self.skipWaiting())
self.addEventListener('activate', e => e.waitUntil(self.clients.claim()))

self.addEventListener('fetch', e => {
  const url = new URL(e.request.url)
  if (!isTileRequest(url)) return

  e.respondWith(
    caches.open(CACHE).then(cache =>
      cache.match(e.request).then(hit => {
        if (hit) return hit
        return fetch(e.request).then(resp => {
          if (resp.ok) cache.put(e.request, resp.clone())
          return resp
        }).catch(() => new Response('', { status: 503 }))
      })
    )
  )
})
