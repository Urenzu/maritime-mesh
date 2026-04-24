# Architecture Decisions

## Storage Engine (Ghost-Log)

**Decision:** Use an existing embedded K/V library for Phase 1; build custom Merkle-LSM in Phase 2.

**Chosen library:** `redb` (pure Rust, MVCC, zero unsafe in public API) or `fjall` (LSM-based, closer to final target).
Prefer `fjall` — its LSM internals mean Phase 2 replacement is surgical rather than a full rewrite.

**Rationale:** Building a custom LSM + WAL + Merkle layer simultaneously with the rest of the stack is too risky for MVP. The Merkle integrity layer and GhostFrame schema are built correctly from day one; only the compaction engine gets swapped.

**Phase 2 target:** Custom LSM with io_uring async I/O, per-block Merkle proofs baked into SSTable format, and compaction-triggered Merkle root gossip.

---

## Frontend Rendering Stack

**Decision:** deck.gl + MapLibre GL JS

**Rationale:**
- deck.gl handles 100k+ entity rendering at 60fps; it is the industry standard for maritime/geospatial C2 use cases.
- Its `@deck.gl/core` WebGPU backend (`WebGPUDevice`) is in active development — we get WebGL2 today with a direct upgrade path to pure WebGPU as the backend stabilizes.
- MapLibre GL JS provides the base nautical chart layer: open source, no API key, offline tile support (critical for vessel ops in low-connectivity zones).
- **Three.js later (Phase N):** deck.gl's `ScenegraphLayer` and `SimpleMeshLayer` consume Three.js GLTF meshes directly. Phase 1 renders vessels as 2D icons; the 3D upgrade swaps in a new layer type without touching the data pipeline.

**NOT chosen for MVP:** Raw WebGPU / Three.js as primary renderer — too much boilerplate for a map-centric use case. Babylon.js — heavier, game-engine-oriented.

---

## AIS Data Source

**Decision:** Real AIS feed (e.g., AISHub, Marine Traffic stream) as primary.

**Lightweight simulation infrastructure** will be built as a secondary path for:
- Integration testing without a live feed
- Replay of recorded dark-activity scenarios for algorithm validation
- CI/CD pipeline testing

Simulation emits the same NMEA/AIS sentence format as the real feed so the ingestion layer is identical.

---

## Dark Zone Threshold

**Decision:** Fully configurable per deployment / per customer.

**Default suggestion:** 15 minutes of AIS gap in a known coverage area triggers a "dark event."

**Factors a customer might tune:**
- Coverage area quality (dense AIS receiver network vs sparse coastal)
- Vessel class (cargo vs fishing vs tanker — each has different AIS reporting intervals by regulation)
- Threat model (counter-smuggling ops want shorter thresholds than routine traffic monitoring)

The gap threshold, minimum confidence score for ghost-prediction, and dead-reckoning horizon should all be runtime-configurable (TOML config, hot-reloadable).

---

## Serialization / Wire Format

**Decision:** Flatbuffers for all GhostFrames (hot path: ingest → WAL → WebGPU).

**Rationale:** Zero-copy memory mapping from the WAL directly into the WebGPU vertex buffer. No deserialization cost on the render path.

**Protobuf** used for mesh gossip messages (Zenoh pub/sub) where zero-copy is less critical and schema evolution matters more.

---

## API Layer

**Decision:** `axum` (Rust) for the HTTP/REST API.

gRPC (`tonic`) will be added in Phase 2 for internal service-to-service communication (ingest → ghost-log, mesh → api).

---

## Cargo Workspace Layout

```
maritime-mesh/
  Cargo.toml              # workspace root
  crates/
    ghost-log/            # WAL + storage engine (fjall-backed Phase 1)
    ingest/               # NMEA/AIS parser → GhostFrame → ghost-log
    api/                  # axum HTTP server
    dark-zone/            # gap detection + Kalman dead-reckoning
    mesh/                 # Zenoh integration (Phase 2)
    sim/                  # lightweight AIS simulator (testing only)
  frontend/               # deck.gl + MapLibre viewer
  schemas/                # Flatbuffer .fbs + Protobuf .proto definitions
  docs/                   # this folder
```
