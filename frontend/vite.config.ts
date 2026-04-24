import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'deck':     ['@deck.gl/core', '@deck.gl/layers', '@deck.gl/aggregation-layers', '@deck.gl/mapbox'],
          'maplibre': ['maplibre-gl'],
        },
      },
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/v1': 'http://localhost:8080',
      '/v1/stream': { target: 'ws://localhost:8080', ws: true },
    },
  },
})
