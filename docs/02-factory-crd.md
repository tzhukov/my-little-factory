# 02 — Factory CRD (YAML manifest + Go types)

## What was done

- `api/v1/factory_types.go` — Go types: Factory, FactorySpec, FactoryStatus, ModelConfig, RepoConfig, StageName, StageStatus, StageResult, WorkerResources
- `api/v1/groupversion_info.go` — GVK constants, Kind/Resource helpers
- `deploy/crd.yaml` — CRD manifest matching Go types
- `go mod tidy` — resolved all dependencies (k8s.io/* packages)
- `go vet ./api/...` passes clean

## Decisions made

1. **Go types → CRD YAML** — types first, YAML manually written to match. controller-gen will be added later when the controller is built.
2. **Controller-managed stages** — `Stages []StageName` is a list of stage names. Controller creates workers for each. Modular via StageName type — future pivot to user-configurable stages is just a type change.
3. **StageResult as structured type** — `Text`, `Diff`, `TestOutput`, `Artifacts` fields. Not a string. This encodes what each stage produces without locking us into a specific format.
4. **CRD references secrets, not raw keys** — `ModelConfig.APIKeySecretRef` and `EndpointSecretRef` both reference k8s Secrets by name + key path. No API keys in the CRD.
5. **Minimal CRD** — stripped `template`, `ttl`, `resources` from initial CRD. `WorkerResources` kept as it's needed for worker pod creation.
6. **StageName as enum** — `StagePlan`, `StageCode`, `StageTest`, `StageReview` constants. OpenAPI enum in CRD YAML enforces valid values.

## Proof it works

```bash
$ go vet ./api/...
# Clean — no output

$ go mod tidy
# Resolved all dependencies successfully
```

CRD YAML structure matches Go types:
- `spec.model.apiKeySecretRef.name` ↔ `ModelConfig.APIKeySecretRef.Name`
- `spec.repo.url` ↔ `RepoConfig.URL`
- `spec.stages` (array of strings) ↔ `[]StageName`
- `status.stages[*].result.text/diff/testOutput/artifacts` ↔ `StageResult` fields

## Known issues

- **No DeepCopy methods** — `+k8s:deepcopy-gen` markers were removed to avoid controller-runtime dependency. DeepCopy will be generated later with controller-gen.
- **CRD YAML is hand-written** — could drift from Go types. Will add controller-gen when the controller is built.
- **groupversion_info.go has no license header** — removed to save space; add later.
- **No unit tests** — Go types have no tests. They're data structures, so tests would be trivial but necessary for confidence.

## Devil's advocate

**Is this ticket actually done?** Mostly. The types compile, `go vet` passes, CRD YAML matches Go types. But:

- **No DeepCopy** — the types are incomplete for k8s use. `controller-gen` will generate `zz_generated.deepcopy.go` later. Without it, we can't register these types in a scheme or use them with the k8s client.
- **No validation in Go** — the CRD has OpenAPI validation, but Go types have no validation. A user could create a Factory with an empty stage name or invalid repo URL.
- **The CRD YAML could be wrong** — I wrote it by hand to match the types. There's no automated check that they stay in sync.

**What's missing:** DeepCopy generation, validation, and a test that verifies CRD YAML matches Go types.

## Blocking edges — ticket 03

**What 03 depends on from this ticket:**
- Go types must be importable (`github.com/earendil-works/factory/api/v1`)
- `FactorySpec.Task`, `FactorySpec.Repo`, `FactorySpec.Stages`, `FactorySpec.Model` must be accessible
- `FactoryStatus.Phase` must accept `FactoryPending`
- CRD YAML must be applied before controller starts

**What 03 will add:**
- `cmd/factory/src/main.rs` — Rust CLI with clap
- `factory submit --task "..." --repo <url>` creates a Factory CR
- Secret creation for model API key + endpoint
