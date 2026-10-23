# Factory — Local AI Software Factory

A minimal, k8s-native software factory that orchestrates AI-assisted development stages across a local cluster. The model is a configurable service — not our problem. The factory is.

---

## 1. Goals

- **Submit a task** ("add pagination to /api/posts") and get back a result (plan, code, tests, review).
- **Run anywhere**: a 4-machine bare-metal k8s cluster or a local Kind cluster for development.
- **Model-agnostic**: any OpenAI-compatible endpoint works. Swap providers via config, not code.
- **Ephemeral workers**: each stage runs in its own isolated pod with a clean repo clone.
- **Minimal surface**: `factory submit --task "..."` and `factory status <id>`. That's the API.

## 2. Architecture

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

## 3. Deployment Modes

### 3.1 Bare-Metal k8s (production)

Deploy `factory-ctrl` to your cluster. Workers schedule to any node.

```bash
# Prerequisites
# - k8s cluster (your 4-machine setup)
# - kubectl pointing to it
# - model API endpoint + key (OpenRouter, Together, etc.)

# Deploy
kubectl apply -f deploy/crd.yaml
kubectl apply -k deploy/kustomization.yaml

# Create model secret
kubectl create secret generic model-key \
  --from-literal=endpoint="https://openrouter.ai/api/v1" \
  --from-literal=model="qwen3.6-35b-a3b" \
  --from-literal=api-key="$OPENROUTER_KEY" \
  --dry-run=client -o yaml | kubectl apply -f -

# Submit a job
factory submit \
  --repo git@github.com:me/myapp.git \
  --task "Add pagination to /api/posts"

# Check status
factory status <job-id>
```

**Worker scheduling:** Workers get no special tolerations — they run as regular pods on any available node. The controller can optionally set node affinity to keep heavy work off the 2c/8g node.

### 3.2 Kind Cluster (development)

A single-command local setup. Everything runs in one Kind cluster on your laptop.

```bash
# Install Kind cluster
kind create cluster --name factory

# Deploy factory into Kind
kubectl apply -f deploy/crd.yaml
kubectl apply -k deploy/kustomization.yaml

# Same workflow — submit, status, get results
factory submit --repo ./myapp --task "Add pagination"
```

**Kind-specific notes:**
- Use `deploy/kind/` manifests (lightweight: no HPA, no network policies, single replica controller)
- Workers run in the same Kind cluster — repo cloning happens inside the cluster via git
- For local repos: use `--repo ./myapp` which the CLI mounts via a hostPath volume (Kind supports this)
- Model API must be reachable from inside the Kind cluster (external HTTP endpoint)

**Kind kustomization overlay:** `deploy/kind/kustomization.yaml` — strips HPA, reduces replicas to 1, removes network policies.

### 3.3 Comparison

| Feature | Bare-Metal | Kind |
|---------|-----------|------|
| Nodes | 4 machines | 1 (Kind cluster) |
| Workers | Distributed across nodes | All in one pod network |
| Repo mount | git clone in pod | hostPath mount (local dev) |
| Controller replicas | 2 (HA) | 1 |
| HPA | Yes | No |
| Network policies | Yes | No |
| Use case | Real work | Development / testing |

## 4. CRD Design

### 4.1 Factory

```yaml
apiVersion: factory.earendil.works/v1
kind: Factory
metadata:
  name: <job-id>
spec:
  model:
    endpoint: string          # e.g., https://openrouter.ai/api/v1
    model: string             # e.g., qwen3.6-35b-a3b
    apiKeySecretRef:          # Secret containing API key
      name: model-key
      key: api-key
    temperature: float        # optional, default 0.7
    maxTokens: int            # optional, default 4096

  repo:
    url: string               # git@github.com:me/myapp.git or file:///home/me/myapp
    branch: string            # optional, default main
    ref: string               # optional, commit/PR/tag

  task: string                # "Add pagination to /api/posts"

  stages:                     # ordered list of stages to run
    - name: plan              # required: plan, code, test, review
    - name: code
    - name: test
    - name: review

  template:                   # optional: custom prompt templates
    plan: |                   # overrides default prompt
      You are a software architect...
    code: |
      You are a senior Go developer...

  resources:                  # worker defaults
    cpu: "1"
    memory: "4Gi"

  ttl:                        # how long to keep results after completion
    hours: 24

status:
  phase: string              # Pending | Running | Complete | Failed
  stages:
    - name: plan
      phase: Complete
      startedAt: "2025-01-01T00:00:00Z"
      completedAt: "2025-01-01T00:01:00Z"
      result:                 # stage output
        plan: |
          1. Add Page struct...
        artifacts:            # files passed to next stage
          plan.md: <base64>
    - name: code
      phase: Complete
      result:
        diff: |
          @@ -1 +1 @@
          ...
    - name: test
      phase: Running
    - name: review
      phase: Pending
  result: string              # final output (diff, summary, etc.)
```

### 4.2 Lifecycle

```
Pending → Running → Complete / Failed
```

1. **Pending**: Factory CR created. Controller hasn't started yet.
2. **Running**: Controller creates worker pods for each stage sequentially.
3. **Complete**: All stages finished. Result available in `status.result`.
4. **Failed**: A stage errored. `status.result` contains error details.

### 4.3 Stage Definitions

| Stage | Purpose | Prompt Template | Input | Output |
|-------|---------|----------------|-------|--------|
| `plan` | Architecture & task breakdown | `ARCHITECT_PROMPT` | task, repo structure | plan document, file list |
| `code` | Generate code changes | `CODE_PROMPT` | plan, repo state | unified diff |
| `test` | Run tests + generate tests | `TEST_PROMPT` | code diff, repo | test output, new test files |
| `review` | Review & suggest improvements | `REVIEW_PROMPT` | code + tests | review comments |

## 5. Components

### 5.1 CLI (`cmd/factory/` — Rust)

```
factory submit   --repo ./myapp --task "..."          # Create Factory CR
factory status <id>                           # Show status
factory logs <id> <stage>                   # Show worker pod logs
factory diff <id>                           # Show final diff
factory cancel <id>                         # Delete Factory CR (stops workers)
```

Rust CLI (clap + k8s Rust client or HTTP). Thin wrapper: creates/reads Factory CRs via k8s API.

### 5.3 Controller (`pkg/factory/` — Go)

The controller reconciles Factory CRs:

```go
type Controller struct {
    k8s     kubernetes.Interface
    model   model.Client
    scheme  *runtime.Scheme
}

func (c *Controller) Reconcile(ctx, factory *Factory) error {
    switch factory.Status.Phase {
    case "": // Pending
        return c.start(factory)
    case "Running":
        return c.progress(factory)
    case "Complete", "Failed":
        return nil // Done
    }
}
```

**Reconciliation flow:**
1. Check which stages are done vs. pending
2. For each pending stage, create a worker pod
3. For each complete stage, read the result
4. When all stages are done, assemble final result
5. Update Factory status

### 5.4 Worker Pod (Rust binary)

Each worker pod runs a Rust binary — the stage is passed as an env var:

```yaml
apiVersion: v1
kind: Pod
metadata:
  generateName: factory-worker-plan-
spec:
  containers:
    - name: worker
      image: factory-worker:latest
      env:
        - name: STAGE_NAME          # "plan", "code", "test", "review"
        - name: MODEL_ENDPOINT
        - name: MODEL_NAME
        - name: MODEL_API_KEY
        - name: FACTORY_TASK
        - name: FACTORY_REPO_URL
        - name: FACTORY_REPO_REF
        - name: PREVIOUS_STAGE_RESULT   # base64 of previous stage output
        - name: PROMPT_TEMPLATE           # base64 of stage prompt
      volumeMounts:
        - name: repo
          mountPath: /workspace
        - name: result
          mountPath: /result
  volumes:
    - name: repo
      # git clone into here, or hostPath mount for Kind
    - name: result
      emptyDir: {}
```

**Worker binary logic:**
1. Clone repo to `/workspace`
2. Read `PREVIOUS_STAGE_RESULT` from env (stage 0: empty)
3. Load `PROMPT_TEMPLATE` from env
4. Call model API with template + task + previous result
5. Write output to `/result/output.txt`
6. Exit 0

### 5.5 Model Client (`pkg/model/` — Go)

```go
package model

// Client is an OpenAI-compatible chat API client.
type Client interface {
    Chat(ctx context.Context, messages []Message) (*Response, error)
}

type Message struct {
    Role    string  // "system", "user", "assistant"
    Content string
}

type Response struct {
    Content string
    Usage   Usage
}

type Usage struct {
    PromptTokens     int
    CompletionTokens int
    TotalTokens      int
}

// NewClient creates a client for any OpenAI-compatible endpoint.
func NewClient(endpoint, model, apiKey string, opts ...Option) Client
```

### 5.6 Stage Prompts (Go)

Default prompts live in `pkg/factory/prompts/`:

```
prompts/
  plan.go    → PlanPrompt
  code.go    → CodePrompt
  test.go    → TestPrompt
  review.go  → ReviewPrompt
```

Each prompt has:
- A **system** message (role, expertise, constraints)
- A **user** template (task + repo structure + previous stage output)
- The `Template` struct:
  ```go
  type Template struct {
      System   string
      User     string       // Go template with {{.Task}}, {{.Repo}}, {{.PreviousResult}}
      MaxTokens int
      Temp     float32
  }
  ```

## 6. Project Structure

```
factory/
├── SPEC.md                      # This file
├── README.md                    # User-facing docs
├── Makefile                     # build, test, deploy, kind
├── go.mod                     # Go module (controller, k8s helpers)
├── Cargo.toml                 # Rust workspace (CLI + worker)
├── rust-toolchain.toml        # Rust toolchain version
├── SPEC.md
├── README.md
├── Makefile
├── cmd/
│   └── factory/               # Rust CLI (clap)
│       └── src/
│           └── main.rs
├── api/
│   └── v1/
│       ├── factory_types.go   # CRD types
│       └── groupversion_info.go
├── pkg/
│   ├── factory/
│   │   ├── controller.go      # Factory controller
│   │   ├── reconciler.go      # Stage-by-stage reconciliation
│   │   ├── prompts/
│   │   │   ├── plan.go
│   │   │   ├── code.go
│   │   │   ├── test.go
│   │   │   └── review.go
│   │   └── worker.go          # Worker pod generation (Go)
│   ├── model/
│   │   ├── client.go          # ModelClient interface
│   │   └── openai.go          # OpenAI-compatible adapter
│   └── k8s/
│       ├── client.go          # k8s client helpers
│       └── secrets.go         # Secret reading
├── worker/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs            # Worker entry point
│       ├── git.rs             # Git clone, diff, status
│       ├── model.rs           # Model client (reqwest + serde)
│       └── prompts.rs         # Prompt rendering
├── deploy/
│   ├── crd.yaml
│   ├── kustomization.yaml
│   ├── k8s/
│   │   ├── controller.yaml
│   │   └── serviceaccount.yaml
│   └── kind/
│       └── kustomization.yaml
└── test/
    └── e2e/
        └── factory_test.go
```

## 7. Build & Deploy

### 7.1 Build

```bash
make build        # Build CLI + worker binary
make crd          # Generate CRD manifests
make docker       # Build worker image
```

### 7.2 Bare-Metal Deploy

```bash
kubectl apply -f deploy/crd.yaml
kubectl apply -k deploy/k8s/
```

### 7.3 Kind Deploy

```bash
kind create cluster --name factory
kubectl apply -f deploy/crd.yaml
kubectl apply -k deploy/kind/
```

### 7.4 Submit a Job

```bash
# Local repo (works in Kind with hostPath mount)
factory submit --repo ./myapp --task "Add pagination to /api/posts"

# Remote repo
factory submit --repo git@github.com:me/myapp.git --task "Add pagination"

# Specific branch
factory submit --repo git@github.com:me/myapp.git --branch feature-x --task "..."

# Custom model
factory submit --repo ./myapp --task "..." \
  --model-endpoint https://openrouter.ai/api/v1 \
  --model qwen3.6-35b-a3b \
  --model-api-key $KEY
```

## 8. Worker Scheduling

### 8.1 Bare-Metal

Workers are scheduled normally by k8s. Optional node affinity:

```yaml
# Keep heavy work off the 2c/8g node
affinity:
  nodeAffinity:
    requiredDuringSchedulingIgnoredDuringExecution:
      nodeSelectorTerms:
        - matchExpressions:
            - key: factory.earendil.works/heavy
              operator: In
              values: ["true"]
```

### 8.2 Kind

All workers run in the Kind cluster. No scheduling concerns.

## 9. Error Handling

| Error | Behavior |
|-------|----------|
| Worker pod fails | Controller recreates pod (up to 3 retries) |
| Model API fails | Worker retries with backoff, then marks stage Failed |
| Repo clone fails | Factory status = Failed, no retries |
| Stage timeout | Configurable per-stage, default 10 min |

## 10. Phase Plan

### Phase 1: Core (MVP)
- [ ] Factory CRD + types
- [ ] Controller (reconcile loop)
- [ ] Worker binary (clone → call model → write result)
- [ ] Model client (OpenAI-compatible)
- [ ] CLI (submit, status)
- [ ] Default prompts (plan, code, test, review)
- [ ] Bare-metal k8s deploy

### Phase 2: Kind Support
- [ ] Kind kustomization overlay
- [ ] hostPath repo mount for local repos
- [ ] E2E test against Kind

### Phase 3: Polish
- [ ] Custom prompt templates
- [ ] Stage timeout config
- [ ] TTL / auto-cleanup
- [ ] `factory diff` output
- [ ] `factory logs` streaming

### Phase 4 (Future)
- [ ] Parallel stage execution (where no dependency)
- [ ] Multi-model routing (plan on cheap model, code on expensive)
- [ ] Git integration (auto-commit, PR creation)
- [ ] Metrics / observability
