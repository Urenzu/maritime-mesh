# maritime-mesh

Decentralized, edge-native maritime Command & Control platform with dark-zone detection.

## Overview

Tracks global vessel traffic via AIS, detects vessels that go dark (stop broadcasting), and uses dead-reckoning to project their last-known trajectory. Built to run on ruggedized edge hardware (vessel-mounted SBCs) as well as cloud infrastructure.

## Architecture

```
AISStream ──► ingest ──► ghost-log (edge persistence)
                    └──► Zenoh mesh ──► api ──► WebSocket ──► frontend
                    └──► QuestDB (cloud analytics / track history)
```

| Crate | Role |
|-------|------|
| `schema` | Canonical `GhostFrame` type shared across all crates |
| `ghost-log` | LSM-backed embedded store + in-process frame bus |
| `mesh` | Zenoh publisher/subscriber wrappers |
| `ingest` | AISStream WebSocket → ghost-log + QuestDB + Zenoh |
| `api` | Axum REST + WebSocket API, dark-zone detection |
| `dark-zone` | Dark event detector + dead-reckoning engine |
| `sim` | AIS traffic simulator for local testing |

**Frontend** — deck.gl + MapLibre, LOD rendering (heatmap → grid → scatter), viewport-filtered WebSocket stream.

## Getting Started

```bash
cp .env.example .env
# fill in AISSTREAM_API_KEY and GFW_API_KEY

cargo run --bin ingest &
cargo run --bin api

cd frontend && npm install && npm run dev
```
