Rundown:
- Initial processing layer for data from the apis (Where we push to zenoh + ghost-log)
- Zenoh for current state of maritime traffic
- Implementing our own custom ghost-log library in rust for the edge (No io_uring for mvp)
- AisStream.io + AISHub (For free websocket maritime data)
- SkyFi + Unseenlabs (For SAR and RF data for dark zone detection)
- We need to pool this data into a unified 2D view.
- Flesh out a mercenary-like brutalist dark command and control SPA
- Primarily rust + typescript + react + three.js later on...
- SQL + potentially other languages to cut down on massive amounts of unneeded rust code at times when we don't need it or its overkill.

Vanguard-OS & Ghost-Log Architecture Plan
1. System Vision: Tactical Truth
A decentralized, edge-native Command and Control (C2) platform designed for intermittent connectivity and "Dark Activity" detection. The system prioritizes local-first forensics over cloud-dependency.

2. Component Architecture
The system is divided into three functional layers to ensure zero-latency situational awareness.

Ingestion Layer (The Vacuum)

Protocol-agnostic proxy (Rust) for NMEA, AIS, and Radar.

Normalizes raw telemetry into immutable Ghost-Frames (Flatbuffers).

Immediate hand-off to local Ghost-Log WAL (Write-Ahead Log).

Mesh Layer (The Backplane)

Zenoh for decentralized mesh networking.

Peer-to-peer synchronization of Merkle-roots between vessels/nodes.

Queryable interface for agents to pull historical state from neighbors.

Presentation Layer (The Map)

WebGPU-accelerated 3D environment.

Zero-copy memory mapping of Ghost-Log data.

Fluid 60fps visualization of 100k+ active and "Ghost" (predicted) entities.

3. Ghost-Log: The Persistence Engine
A high-performance, embedded LSM-tree optimized for edge hardware and cryptographic integrity.

The Merkle-LSM Structure

MemTable: Lock-free SkipList in RAM for million-plus writes per second.

SSTables: Immutable, sorted blocks on disk with integrated Merkle-trees for tamper-proof history.

Bloom Filters: Probabilistic data structures for O(1) membership checks of known/unknown vessels.

Agentic Analytics (The "Ghost" State)

Decoupled observer loop monitoring the LSM compaction.

Automatic injection of Synthetic Frames when telemetry gaps are detected.

Vectorized motion models (Kalman/Constant Velocity) used as "filling" for Dark Activity forensics.

4. Data Infrastructure & Storage Tiering
Optimizing for "Forensic Truth" across variable storage constraints.

Hot Tier (Tactical Edge)

Ghost-Log (Rust): Persistent WAL + LSM on local NVMe.

Memory-mapped Flatbuffers for the WebGPU frontend.

Warm Tier (Regional Analytics)

QuestDB: High-ingestion time-series store for real-time SQL queries across the local mesh.

Maintains the "Last Known Good" state for all vessels in the sector.

Cold Tier (Global Archive)

Parquet + Object Store (S3/R2): SSTable fragments are sealed and shipped as columnar Parquet files.

Enables massive-scale forensic analysis of historical Dark Activity patterns.

5. Implementation Milestones (Brief)
Phase 1: Core Ghost-Log WAL implementation with io_uring and Flatbuffer framing.

Phase 2: Zenoh integration for decentralized Merkle-root gossip and gap-filling.

Phase 3: Agentic "Shadow State" engine for autonomous dead-reckoning during network dropouts.

Phase 4: WebGPU visualization layer with direct buffer-mapping from the Ghost-Log core.
