import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'deck':     ['@deck.gl/core', '@deck.gl/layers', '@deck.gl/mapbox'],
          'maplibre': ['maplibre-gl'],
          'pmtiles':  ['pmtiles'],
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
