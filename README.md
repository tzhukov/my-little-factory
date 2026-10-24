# Factory

A minimal, k8s-native AI software factory. Submit a task, get back a pipeline of AI-assisted stages (plan → code → test → review). The model is a configurable service — not our problem.

## Status

| # | Ticket | Status |
|---|--------|--------|
| 01 | Project scaffold: go.mod + Cargo workspace + Makefile | ✅ |
| 02 | Factory CRD (YAML manifest + Go types) | ✅ |
| 03 | `factory submit` CLI (Rust) | ✅ |
| 04 | Controller watches CRs, logs Pending | ✅ |
| 05 | Kind cluster setup + deploy | ✅ |
| 06 | Controller creates worker pod for first stage | ✅ |
| 07 | Worker binary (Rust) — clone repo + call model | ✅ |
| 08 | Controller collects stage result → updates status | ✅ |
| 09 | Full sequential pipeline (all 4 stages) | ✅ |
| 10 | `factory status` and `factory diff` CLI | ✅ |

## Quick Start

```bash
# Bare-metal k8s
kubectl apply -f deploy/crd.yaml
kubectl apply -k deploy/k8s/
factory submit --repo ./myapp --task "Add pagination"

# Kind (local dev)
kind create cluster --name factory
kubectl apply -f deploy/crd.yaml
kubectl apply -k deploy/kind/
factory submit --repo ./myapp --task "Add pagination"
```

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                      k8s Cluster                             │
│                                                              │
│  ┌──────────────────┐                                        │
│  │ factory-ctrl     │  Controller: watches Factory CRs,      │
│  │                  │  creates worker pods, collects results │
│  └────────┬─────────┘                                        │
│           │                                                    │
│           ▼                                                    │
│  ┌──────────────────────────────────────────────────┐         │
│  │  Factory CR (user submits a job)                 │         │
│  │  Spec: task, repo, stages, model config          │         │
│  └───────────────┬──────────────────────────────────┘         │
│                  │                                             │
│        ┌────────┼────────┬────────┐                           │
│        ▼        ▼        ▼        ▼                           │
│   ┌────────┐ ┌────────┐ ┌────────┐ ┌────────┐               │
│   │plan    │ │code    │ │test    │ │review  │               │
│   │worker  │ │worker  │ │worker  │ │worker  │               │
│   │pod     │ │pod     │ │pod     │ │pod     │               │
│   └────────┘ └────────┘ └────────┘ └────────┘               │
│                                                              │
│  External: model API (endpoint + key from Secret)            │
└──────────────────────────────────────────────────────────────┘
```

## Tech Stack

| Component | Language |
|-----------|----------|
| Controller (Go) | Go |
| Worker binary | Rust |
| CLI | Rust |
| CRD types | Go |

## Docs

- [SPEC.md](SPEC.md) — Full specification
