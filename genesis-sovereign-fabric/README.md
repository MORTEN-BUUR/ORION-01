# GENESIS Sovereign Fabric

Production-grade, sovereign AI Control Plane. Deterministic. Audit-safe. Hardware-bound. Federated.

## Components

- **gsf-core**: AuditChain, SymbolMap, StateMachine, SignedLedger, ReplayEngine, DeterministicExecutor
- **gsf-policy**: YAML DSL, Scope Enforcement, Invariants, Action Validator
- **gsf-api**: Axum HTTP API (`/run`, `/mesh/sync`, `/audit/export`, `/audit/verify`, `/hardware/attest`, `/system/status`)
- **gsf-mesh**: mTLS peer verification, Peer allowlist, Fork resolution
- **gsf-registry**: ModelRegistry, SBOM hash storage, Governance flags
- **gsf-hardware**: Enclave abstraction, Remote attestation
- **gsf-observability**: Prometheus metrics, Structured logs
- **gsf-cli**: verify, replay, inspect, export-ledger
- **Python SDK**: execute(), verify_audit(), export_chain(), mesh_sync()

## Quick Start

```bash
# Build
cargo build --release

# Run API server
GSF_BIND_ADDR=0.0.0.0:3000 cargo run -p gsf-api --bin gsf-server

# CLI
cargo run -p gsf-cli -- verify --ledger-path ledger.json
```

## Python SDK

```bash
cd python && pip install -e .
```

```python
from gsf_sdk import execute, export_chain, GsfClient

client = GsfClient("http://localhost:3000")
result = client.execute("run", {"domain": "test"})
chain = client.export_chain()
```

## EU AI Act Compliance (gsf-euaiact)

- **Risk Classification** (Article 6, Annex III): Unacceptable, High, Limited, Minimal
- **Human Oversight** (Article 14): Human-in-the-loop, Human-on-the-loop, Human-in-command
- **Technical Documentation** (Annex IV): Structure for high-risk systems
- **Transparency** (Chapter IV): End-user disclosure, interaction notices
- **Fundamental Rights Impact** (Article 29): Impact assessment structure

### API Endpoints

| Endpoint | Description |
|----------|-------------|
| `POST /oversight/submit` | Submit action for human approval |
| `POST /oversight/approve` | Approve pending decision (executes) |
| `POST /oversight/halt` | Halt/override pending decision |
| `GET /oversight/pending` | List pending decisions |
| `POST /registry/register` | Register model with risk level, Annex III category |
| `POST /euaiact/disclosure` | Get end-user disclosure for risk level |

## Genesis Anchor

```
sha256:acb92fd8346a65ff17dbf9a41e3003f2d566a17f839af4c3a90a4b4b1789dd28a
```
