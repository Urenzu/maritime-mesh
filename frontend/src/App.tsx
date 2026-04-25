import { useEffect, useRef, useState, useCallback } from 'react'
import { initMap, buildLayers, ZOOM_HEATMAP, ZOOM_GRID } from './map'
import { fetchVessels, fetchDarkZones, fetchVesselDark } from './api'
import type { DarkEvent } from './types'
import type { VesselSnapshot } from './worker-types'
import Hud from './components/Hud'
import VesselPanel from './components/VesselPanel'

function getLod(zoom: number): number {
  if (zoom < ZOOM_HEATMAP) return 0
  if (zoom < ZOOM_GRID)    return 1
  return 2
}

export default function App() {
  const containerRef = useRef<HTMLDivElement>(null)

  // ── React UI state ──────────────────────────────────────────────────────────
  const [vesselCount, setVesselCount] = useState(0)
  const [darkCount,   setDarkCount]   = useState(0)
  const [lastUpdate,  setLastUpdate]  = useState('—')
  const [panelOpen,   setPanelOpen]   = useState(false)
  const [panelReport, setPanelReport] = useState<Record<string, unknown> | null>(null)

  // ── Mutable refs for the rAF loop (bypass React re-renders) ────────────────
  const posRef      = useRef(new Float32Array(0))
  const colRef      = useRef(new Uint8Array(0))
  const msiRef      = useRef(new Int32Array(0))
  const vcRef       = useRef(0)
  const dcRef       = useRef(0)
  const dataDirty   = useRef(false)
  const currentLod  = useRef(-1)
  const darkEvents  = useRef<DarkEvent[]>([])

  const onVesselClick = useCallback(async (mmsi: number) => {
    try {
      const report = await fetchVesselDark(mmsi) as Record<string, unknown>
      setPanelReport(report)
      setPanelOpen(true)
    } catch (e) { console.error('vessel report failed', e) }
  }, [])

  // Stable ref so the rAF loop always calls the latest callback
  const onVesselClickRef = useRef(onVesselClick)
  onVesselClickRef.current = onVesselClick

  useEffect(() => {
    if (!containerRef.current) return

    const { overlay, getZoom, onViewportChange, onZoomEnd, remove } =
      initMap(containerRef.current)

    // ── Worker ────────────────────────────────────────────────────────────────
    const worker = new Worker(
      new URL('./vessel-worker.ts', import.meta.url),
      { type: 'module' },
    )

    worker.onmessage = ({ data }: MessageEvent<VesselSnapshot>) => {
      posRef.current   = data.positions
      colRef.current   = data.colors
      msiRef.current   = data.mmsis
      vcRef.current    = data.count
      dcRef.current    = data.darkCount
      dataDirty.current = true
    }

    worker.postMessage({
      type:  'connect',
      wsUrl: `${location.protocol === 'https:' ? 'wss' : 'ws'}://${location.host}/v1/stream`,
    })

    fetchVessels()
      .then(list => worker.postMessage({ type: 'seed', vessels: list }))
      .catch(console.error)

    // ── Dark zone poll ────────────────────────────────────────────────────────
    async function refreshDarkZones() {
      try {
        const { dark_events } = await fetchDarkZones()
        darkEvents.current = dark_events.filter(e => e.is_ongoing)
        dataDirty.current  = true
      } catch { /* keep last state */ }
    }
    refreshDarkZones()
    const dzInterval = setInterval(refreshDarkZones, 30_000)

    // ── Viewport → worker ─────────────────────────────────────────────────────
    let vpTimer: ReturnType<typeof setTimeout> | undefined
    onViewportChange(bbox => {
      clearTimeout(vpTimer)
      vpTimer = setTimeout(() => worker.postMessage({ type: 'set-viewport', bbox }), 300)
    })

    onZoomEnd(() => {
      const lod = getLod(getZoom())
      if (lod !== currentLod.current) {
        currentLod.current = lod
        dataDirty.current  = true
      }
    })

    // ── rAF loop ──────────────────────────────────────────────────────────────
    let rafId: number
    const loop = () => {
      if (dataDirty.current) {
        dataDirty.current = false
        const zoom = getZoom()
        currentLod.current = getLod(zoom)
        overlay.setProps({
          layers: buildLayers(
            vcRef.current,
            posRef.current,
            colRef.current,
            msiRef.current,
            darkEvents.current,
            zoom,
            onVesselClickRef.current,
          ),
        })
        setVesselCount(vcRef.current)
        setDarkCount(dcRef.current)
        setLastUpdate(new Date().toISOString().slice(11, 19) + ' UTC')
      }
      rafId = requestAnimationFrame(loop)
    }
    rafId = requestAnimationFrame(loop)

    return () => {
      cancelAnimationFrame(rafId)
      clearInterval(dzInterval)
      clearTimeout(vpTimer)
      worker.terminate()
      remove()
    }
  }, []) // mount once

  return (
    <>
      <div ref={containerRef} className="map-root" />
      <Hud vesselCount={vesselCount} darkCount={darkCount} lastUpdate={lastUpdate} />
      <VesselPanel
        open={panelOpen}
        report={panelReport}
        onClose={() => setPanelOpen(false)}
      />
    </>
  )
}
