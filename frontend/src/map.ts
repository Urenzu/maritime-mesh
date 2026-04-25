import maplibregl from 'maplibre-gl'
import { Protocol } from 'pmtiles'
import { MapboxOverlay } from '@deck.gl/mapbox'
import { ScatterplotLayer } from '@deck.gl/layers'
import type { DarkEvent } from './types'
import type { ViewportBbox } from './api'

// Register pmtiles:// protocol with MapLibre before any map is constructed.
const pmtilesProtocol = new Protocol()
maplibregl.addProtocol('pmtiles', pmtilesProtocol.tile.bind(pmtilesProtocol))

const COLOR_DARK:      [number, number, number]    = [255, 60,  60 ]
const COLOR_DARK_ZONE: [number, number, number, number] = [255, 60, 60, 45]

export const ZOOM_HEATMAP = 5
export const ZOOM_GRID    = 8

// ── Styles ────────────────────────────────────────────────────────────────────

const PMTILES_URL = import.meta.env.VITE_PMTILES_URL as string | undefined

function maritimeVectorStyle(url: string): maplibregl.StyleSpecification {
  return {
    version: 8,
    sources: {
      protomaps: {
        type: 'vector',
        url: `pmtiles://${url}`,
        attribution: '© <a href="https://protomaps.com">Protomaps</a> © <a href="https://openstreetmap.org">OpenStreetMap</a>',
      },
    },
    layers: [
      { id: 'background', type: 'background',  paint: { 'background-color': '#091520' } },
      { id: 'land-ne',    type: 'fill', source: 'protomaps', 'source-layer': 'natural_earth', paint: { 'fill-color': '#1a1a22' } },
      { id: 'land',       type: 'fill', source: 'protomaps', 'source-layer': 'earth',         paint: { 'fill-color': '#1a1a22' } },
      { id: 'water',      type: 'fill', source: 'protomaps', 'source-layer': 'water',         paint: { 'fill-color': '#091520' } },
      { id: 'borders',    type: 'line', source: 'protomaps', 'source-layer': 'boundaries',    paint: { 'line-color': '#2e2e3e', 'line-width': 0.5 } },
    ],
  }
}

const MARITIME_RASTER_STYLE: maplibregl.StyleSpecification = {
  version: 8,
  sources: {
    'carto-dark': {
      type: 'raster',
      tiles: [
        'https://a.basemaps.cartocdn.com/dark_nolabels/{z}/{x}/{y}.png',
        'https://b.basemaps.cartocdn.com/dark_nolabels/{z}/{x}/{y}.png',
        'https://c.basemaps.cartocdn.com/dark_nolabels/{z}/{x}/{y}.png',
        'https://d.basemaps.cartocdn.com/dark_nolabels/{z}/{x}/{y}.png',
      ],
      tileSize: 256,
      attribution: '© <a href="https://carto.com/attributions">CARTO</a> © <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
      maxzoom: 19,
    },
  },
  layers: [{
    id: 'carto-dark-tiles',
    type: 'raster',
    source: 'carto-dark',
    paint: { 'raster-fade-duration': 0 },
  }],
}

// ── Map init ──────────────────────────────────────────────────────────────────

export function initMap(container: HTMLElement): {
  overlay: MapboxOverlay
  getZoom: () => number
  onViewportChange: (cb: (bbox: ViewportBbox) => void) => void
  onZoomEnd: (cb: () => void) => void
} {
  const map = new maplibregl.Map({
    container,
    style: PMTILES_URL ? maritimeVectorStyle(PMTILES_URL) : MARITIME_RASTER_STYLE,
    center: [2.5, 51.5],
    zoom: 3,
    antialias: false,
    fadeDuration: 0,
    attributionControl: false,
    maxTileCacheSize: 200,
    renderWorldCopies: false,
    refreshExpiredTiles: false,
    pixelRatio: Math.min(window.devicePixelRatio, 1.5),
  })

  map.addControl(new maplibregl.AttributionControl({ compact: true }), 'bottom-right')
  map.addControl(new maplibregl.NavigationControl(), 'top-right')
  map.addControl(new maplibregl.ScaleControl({ unit: 'nautical' }), 'bottom-left')

  map.on('load', () => {
    map.addSource('openseamap', {
      type: 'raster',
      tiles: ['https://tiles.openseamap.org/seamark/{z}/{x}/{y}.png'],
      tileSize: 256,
      attribution: '© <a href="https://www.openseamap.org">OpenSeaMap</a>',
      minzoom: 8,
    })
    map.addLayer({
      id: 'openseamap-layer',
      type: 'raster',
      source: 'openseamap',
      minzoom: 8,
      paint: { 'raster-opacity': 0.8, 'raster-fade-duration': 0 },
    })
  })

  // interleaved: true — deck.gl renders inside MapLibre's GL context.
  // Single WebGL context, single render loop, no canvas compositing overhead.
  const overlay = new MapboxOverlay({ interleaved: true, layers: [] })
  map.addControl(overlay as unknown as maplibregl.IControl)

  let vpCallback: ((bbox: ViewportBbox) => void) | null = null
  const emitViewport = () => {
    if (!vpCallback) return
    const b = map.getBounds()
    vpCallback({ min_lat: b.getSouth(), min_lon: b.getWest(), max_lat: b.getNorth(), max_lon: b.getEast() })
  }
  map.on('moveend', emitViewport)
  map.on('zoomend', emitViewport)

  return {
    overlay,
    getZoom:          () => map.getZoom(),
    onViewportChange: (cb) => { vpCallback = cb },
    onZoomEnd:        (cb) => { map.on('zoomend', cb) },
  }
}

// ── Layer builder ─────────────────────────────────────────────────────────────
// Typed array data path: no per-vessel JS function calls, buffers go straight
// to GPU attribute upload.

export function buildLayers(
  count:          number,
  positions:      Float32Array,
  colors:         Uint8Array,
  mmsis:          Int32Array,
  darkEvents:     DarkEvent[],
  zoom:           number,
  onVesselClick:  (mmsi: number) => void,
) {
  const layers = []

  if (zoom >= ZOOM_GRID) {
    layers.push(new ScatterplotLayer<DarkEvent>({
      id: 'dark-zones',
      data: darkEvents.filter(e => e.is_ongoing),
      getPosition: d => [d.last_lon, d.last_lat],
      getRadius: 15_000,
      getFillColor: COLOR_DARK_ZONE,
      getLineColor: COLOR_DARK,
      stroked: true,
      lineWidthMinPixels: 1,
      radiusUnits: 'meters',
      pickable: false,
    }))
  }

  layers.push(new ScatterplotLayer({
    id: 'vessels',
    data: { length: count },
    attributes: {
      getPosition: { value: positions, size: 2 },
      getFillColor: { value: colors,    size: 4, normalized: false },
    },
    getRadius:       zoom < ZOOM_HEATMAP ? 60_000 : zoom < ZOOM_GRID ? 8_000 : 400,
    radiusUnits:     'meters',
    radiusMinPixels: zoom < ZOOM_HEATMAP ? 1 : 2,
    radiusMaxPixels: zoom < ZOOM_HEATMAP ? 3 : zoom < ZOOM_GRID ? 6 : 12,
    opacity:         zoom < ZOOM_HEATMAP ? 0.5 : zoom < ZOOM_GRID ? 0.8 : 1.0,
    stroked:         zoom >= ZOOM_GRID,
    getLineColor:    [255, 255, 255, 50],
    lineWidthMinPixels: 1,
    pickable:        zoom >= ZOOM_GRID,
    onClick: (info) => {
      if (info.index >= 0 && info.index < mmsis.length) {
        onVesselClick(mmsis[info.index])
      }
    },
  }))

  return layers
}
