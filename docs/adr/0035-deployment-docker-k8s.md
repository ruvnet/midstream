# 0035 — Deployment shape: one Dockerfile + a Helm chart

- **Status:** Proposed
- **Date:** 2026-05-13
- **Deciders:** @ruv
- **Tags:** deployment, docker, kubernetes

## Context and Problem Statement

Deployment artefacts today are scattered and AIMDS-centric:

| Path                          | Content                                                                                          |
|-------------------------------|--------------------------------------------------------------------------------------------------|
| `AIMDS/docker-compose.yml`    | 121 lines; services for `redis`, `agentdb`, `lean-server` (leanprover/lean4 image), AIMDS gateway/node |
| `AIMDS/docker/Dockerfile.gateway` | Gateway container                                                                            |
| `AIMDS/docker/Dockerfile.node` | Node container                                                                                  |
| `AIMDS/docker/Dockerfile.rust` | Rust build container                                                                            |
| `AIMDS/docker/prometheus.yml`  | Prometheus scrape config                                                                        |
| `AIMDS/k8s/deployment.yaml`    | 28 lines; single-replica Deployment                                                             |
| `AIMDS/k8s/configmap.yaml`     | 13 lines                                                                                        |
| `AIMDS/k8s/service.yaml`       | 12 lines                                                                                        |
| **(repo root)**                | **no Dockerfile, no Helm chart, no compose, no k8s manifests for midstream itself**             |

Effects:

- **There is no shippable midstream container.** The published Rust
  binary `midstream` cannot be `docker pull`-ed because nothing
  builds an image for it.
- **AIMDS's deployment story is silently overloaded as midstream's.**
  Anyone reading `AIMDS/docker-compose.yml` discovers a `lean-server`
  (Lean theorem prover image, unrelated to midstream's
  `crates/strange-loop`) and an `agentdb/agentdb:latest` image which
  is neither in this repo nor pinned by digest.
- **`agentdb:latest`, `redis:7-alpine`, `leanprover/lean4:latest`** —
  three unpinned `:latest` tags in `AIMDS/docker-compose.yml`. Any
  upstream rebuild breaks reproducibility.
- **K8s deployment is single-replica with no readiness, no liveness,
  no PDB, no HPA, no NetworkPolicy** (`AIMDS/k8s/deployment.yaml` is
  28 lines — that's roughly the spec stub Kubernetes shows in
  tutorials).
- **No published image registry strategy.** The release workflow
  ([ADR-0017](0017-release-and-publishing.md)) does not push a
  container image anywhere.

## Decision Drivers

- **One canonical container** for midstream that runs the binary on
  any Docker-capable host without bringing the AIMDS service mesh.
- **Pin everything.** Base images by digest, helper services by
  version, with a documented bump cadence.
- **K8s manifests with the real production knobs** (probes, resource
  limits, NetworkPolicy, PDB, HPA where appropriate).
- **Helm chart, not raw YAML.** Composition of the AIMDS sidecar,
  observability sidecar (per [ADR-0010](0010-allocator-observability.md)),
  and provider env should be templated.
- **Image publishing in the release workflow.**

## Considered Options

1. **Status quo.** AIMDS-only deployment artefacts; no midstream
   image.
2. **Add `deploy/docker/Dockerfile` for midstream + reuse
   `AIMDS/docker-compose.yml` as the multi-service shape.** Cheapest
   way to fill the gap.
3. **Add `deploy/docker/Dockerfile` for midstream + a Helm chart at
   `deploy/helm/midstream/` covering the whole stack (midstream
   binary, AIMDS sidecar, observability collector). Move the existing
   AIMDS Docker/k8s artefacts under `deploy/` to consolidate.**
4. **Externalize deployment.** Move all Docker/k8s artefacts to a
   sibling repo `midstream-deploy`. Cleanest separation; loses
   lockstep with code.

## Decision Outcome

**Chosen option: Option 3 — add the midstream container + Helm
chart; consolidate under `deploy/`.**

New layout:

```
deploy/
├── docker/
│   ├── Dockerfile                # multi-stage: chef → builder → runtime
│   ├── docker-compose.yml        # midstream + redis + otlp-collector
│   └── docker-compose.dev.yml    # adds AIMDS, lean-server (for full-stack dev)
└── helm/midstream/
    ├── Chart.yaml
    ├── values.yaml               # documented defaults
    ├── values-prod.yaml.example
    └── templates/
        ├── deployment.yaml       # probes, requests/limits, securityContext
        ├── service.yaml
        ├── configmap.yaml        # references Secret for API keys
        ├── secret.yaml.example
        ├── pdb.yaml              # PodDisruptionBudget
        ├── hpa.yaml              # optional autoscaler
        ├── networkpolicy.yaml    # restrict egress per ADR-0015 spirit
        └── servicemonitor.yaml   # Prometheus-Operator scrape config
```

Dockerfile rules:

- Multi-stage build with `cargo-chef` for layer caching.
- `FROM rust:1.81-slim` (pinned by digest) → builder.
- `FROM gcr.io/distroless/cc-debian12` (pinned by digest) → runtime.
- Non-root user (`USER 65532:65532`).
- Binary at `/usr/local/bin/midstream`.
- `ENTRYPOINT ["/usr/local/bin/midstream"]`, no shell.
- `HEALTHCHECK` against the `midstream://status` MCP resource (per
  [ADR-0032](0032-mcp-tool-surface.md)).
- Image size target: ≤ 80 MB.

Helm chart rules:

- `securityContext.runAsNonRoot: true`, `readOnlyRootFilesystem: true`,
  `allowPrivilegeEscalation: false`, `capabilities.drop: [ALL]`.
- `livenessProbe` and `readinessProbe` against the `:8080/healthz`
  endpoint (separate from MCP; HTTP for k8s convenience).
- `resources.requests` and `resources.limits` mandatory; no
  unrestricted pods.
- `NetworkPolicy` denies egress except to:
  - the configured LLM provider hosts (from `ProvidersConfig`, per
    [ADR-0019](0019-config-system.md)),
  - the OTLP collector (per [ADR-0010](0010-allocator-observability.md)),
  - DNS.
- `ServiceMonitor` ships out-of-box for prom-operator clusters.

Image publishing:

- Release workflow ([ADR-0017](0017-release-and-publishing.md)) gains
  a step that builds the Dockerfile and pushes:
  - `ghcr.io/ruvnet/midstream:vX.Y.Z`
  - `ghcr.io/ruvnet/midstream:X.Y` (minor track)
  - `ghcr.io/ruvnet/midstream:latest` (only on stable releases per
    [ADR-0024](0024-semver-and-api-stability.md)).
- Image is signed via `cosign` and attested via
  `actions/attest-build-provenance@v1` (already used for crates).
- The Helm chart is packaged and published to a GH Pages helm repo
  alongside docs (`gh-pages` branch under `/charts/`).

Migration of AIMDS artefacts:

- `AIMDS/docker/`, `AIMDS/docker-compose.yml`, `AIMDS/k8s/` move
  under `deploy/aimds/`. Same content, different path.
- `:latest` tags in the compose files are replaced by pinned
  versions (`agentdb/agentdb:0.X.Y`, `redis:7.4-alpine`,
  `leanprover/lean4:v4.X.Y`) — each with a comment listing the bump
  cadence and the next-review date.

### Positive consequences

- One canonical, signed, attested container image for midstream
  exists.
- Helm chart with real production knobs.
- AIMDS deployment artefacts stop being mistaken for midstream's;
  they live under `deploy/aimds/`.
- Image publishing is part of the release workflow, not an
  out-of-band action.

### Negative consequences

- Significant one-time work. The Helm chart is non-trivial;
  ~200–400 lines of YAML to write carefully.
- The release workflow gets slower (image build adds ~3–5 minutes).
  Mitigated by Buildx layer cache.
- Anyone with deep links into `AIMDS/k8s/*.yaml` from external
  infra breaks. Mitigated by leaving a stub README at the old paths
  pointing at the new location.

## Implementation notes

- Land this ADR; do the migration in a follow-up PR
  `chore: consolidate deploy under deploy/ per ADR-0035`.
- Add `deploy/docker/Dockerfile` with the multi-stage shape above.
- Write the Helm chart from a `helm create midstream` scaffold;
  override every template.
- Add the image-build step to `.github/workflows/release.yml`:
  `docker/build-push-action@v6`, `sigstore/cosign-installer@v3`,
  `actions/attest-build-provenance@v1`.
- Document the deployment shape in `docs/DEPLOYMENT.md` (one of the
  4 kept-canonical docs per [ADR-0020](0020-docs-triage.md)).

## Links

- Related: [ADR-0010](0010-allocator-observability.md),
  [ADR-0014](0014-supply-chain-pinning.md),
  [ADR-0015](0015-wasm-egress-allowlist.md),
  [ADR-0017](0017-release-and-publishing.md),
  [ADR-0019](0019-config-system.md),
  [ADR-0020](0020-docs-triage.md),
  [ADR-0024](0024-semver-and-api-stability.md),
  [ADR-0032](0032-mcp-tool-surface.md).
- `cargo-chef`: https://github.com/LukeMathWalker/cargo-chef
- distroless: https://github.com/GoogleContainerTools/distroless
- cosign: https://docs.sigstore.dev/cosign/
