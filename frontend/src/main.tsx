import 'maplibre-gl/dist/maplibre-gl.css'
import './style.css'
import { createRoot } from 'react-dom/client'
import App from './App'

if ('serviceWorker' in navigator) {
  navigator.serviceWorker.register('/sw.js').catch(() => {/* non-fatal */})
}

createRoot(document.getElementById('app')!).render(<App />)
