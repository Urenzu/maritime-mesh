import maplibregl from 'maplibre-gl'
import { MapboxOverlay } from '@deck.gl/mapbox'
import { ScatterplotLayer } from '@deck.gl/layers'
import { HeatmapLayer, GridLayer } from '@deck.gl/aggregation-layers'
import type { VesselState, DarkEvent } from './types'
import type { ViewportBbox } from './api'

const COLOR_REAL:      [number, number, number] = [0,   210, 255]
const COLOR_SYNTHETIC: [number, number, number] = [255, 180, 0  ]
const COLOR_DARK:      [number, number, number] = [255, 60,  60 ]
const COLOR_DARK_ZONE: [number, number, number, number] = [255, 60, 60, 45]

export const ZOOM_HEATMAP = 5
export const ZOOM_GRID    = 8

export function initMap(
  container: HTMLElement,
  onVesselClick: (v: VesselState) => void,
): {
  overlay: MapboxOverlay
  getZoom: () => number
  onViewportChange: (cb: (bbox: ViewportBbox) => void) => void
  onZoomEnd: (cb: () => void) => void
} {
  const DARK_RASTER_STYLE: maplibregl.StyleSpecification = {
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

  const map = new maplibregl.Map({
    container,
    style: DARK_RASTER_STYLE,
    center: [2.5, 51.5],
    zoom: 3,
    antialias: false,
    fadeDuration: 0,
    attributionControl: false,
    maxTileCacheSize: 200,
    renderWorldCopies: false,
    refreshExpiredTiles: false,
    maxParallelImageRequests: 32,
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

  const overlay = new MapboxOverlay({
    interleaved: false,
    layers: [],
    // Pass vessel click up.
    onClick: (info) => {
      if (info.object) onVesselClick(info.object as VesselState)
    },
  })
  map.addControl(overlay as unknown as maplibregl.IControl)

  let vpCallback: ((bbox: ViewportBbox) => void) | null = null

  const emitViewport = () => {
    if (!vpCallback) return
    const b = map.getBounds()
    vpCallback({
      min_lat: b.getSouth(),
      min_lon: b.getWest(),
      max_lat: b.getNorth(),
      max_lon: b.getEast(),
    })
  }

  map.on('moveend', emitViewport)
  map.on('zoomend', emitViewport)

  return {
    overlay,
    getZoom: () => map.getZoom(),
    onViewportChange: (cb) => { vpCallback = cb },
    onZoomEnd:        (cb: () => void) => { map.on('zoomend', cb) },
  }
}

export function buildLayers(
  vessels: VesselState[],
  darkEvents: DarkEvent[],
  zoom: number,
) {
  if (zoom < ZOOM_HEATMAP) {
    return [
      new HeatmapLayer<VesselState>({
        id: 'vessel-heat',
        data: vessels,
        getPosition: v => [v.lon, v.lat],
        getWeight: v => v.is_dark ? 3 : 1,
        radiusPixels: 30,
        intensity: 1,
        threshold: 0.05,
        colorRange: [
          [0,   20,  80,  0  ],
          [0,   80,  180, 120],
          [0,   210, 255, 180],
          [255, 180, 0,   200],
          [255, 60,  60,  240],
          [255, 0,   0,   255],
        ],
      }),
    ]
  }

  if (zoom < ZOOM_GRID) {
    return [
      new GridLayer<VesselState>({
        id: 'vessel-grid',
        data: vessels,
        getPosition: v => [v.lon, v.lat],
        cellSize: 50_000,
        extruded: false,
        pickable: false,
        colorRange: [
          [0,   40,  100],
          [0,   100, 200],
          [0,   200, 255],
          [255, 200, 0  ],
          [255, 100, 0  ],
          [255, 0,   0  ],
        ],
      }),
    ]
  }

  return [
    new ScatterplotLayer<DarkEvent>({
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
    }),
    new ScatterplotLayer<VesselState>({
      id: 'vessels',
      data: vessels,
      getPosition: v => [v.lon, v.lat],
      getRadius: 400,
      getFillColor: v => {
        if (v.is_dark)          return COLOR_DARK
        if (v.confidence < 1.0) return COLOR_SYNTHETIC
        return COLOR_REAL
      },
      getLineColor: [255, 255, 255, 50],
      stroked: true,
      lineWidthMinPixels: 1,
      radiusUnits: 'meters',
      radiusMinPixels: 2,
      radiusMaxPixels: 12,
      pickable: true,
    }),
  ]
}
