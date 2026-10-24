# 03 — `factory submit` CLI (Rust)

## What was done

- `cmd/factory/src/main.rs` — Rust CLI with `submit` and `install` subcommands
- `cmd/factory/Cargo.toml` — dependencies: `clap`, `kube`, `reqwest`, `serde`, `uuid`, `base64`, `serde_yaml`, `anyhow`, `http`, `secrecy`
- `deploy/k8s/controller.yaml` — placeholder for controller deployment (filled by ticket 04)

## CLI commands

### `factory submit`

```bash
factory submit \
  --task "Add user authentication" \
  --repo "git@github.com:org/repo.git" \
  --branch main \
  --model "gpt-4" \
  --model-api-key "sk-..." \
  --model-endpoint "https://api.openai.com/v1"
```

Interactive prompts when flags not provided:
```bash
factory submit --task "..." --repo "..."
# Prompts: Model API key, Model endpoint
```

### `factory install`

```bash
factory install
# Creates: namespace 'factory', CRD, controller deployment
```

### Shared flags

- `--kubeconfig` — custom kubeconfig path (NOTE: not yet supported in kube 0.93; use `KUBECONFIG` env var instead)
- `--namespace` — namespace (default: `factory`)

## Decisions made

1. **`kube` crate for config loading** — used `Config::infer()` to load kubeconfig. Raw `reqwest` for API calls (avoids kube 0.93 API churn).
2. **`--kubeconfig` not yet supported** — kube 0.93's public API doesn't allow custom kubeconfig paths. Users must use `KUBECONFIG` env var.
3. **Interactive prompts** — falls back to env var (e.g., `MODEL_API_KEY`), then prompts. This is script-friendly (flags/env) but also usable interactively.
4. **`factory install`** — CLI subcommand that creates namespace, applies CRD, and deploys controller placeholder.
5. **`base64::engine::general_purpose::STANDARD.encode()`** — explicit engine to avoid deprecated `base64::encode`.
6. **`secrecy::ExposeSecret`** — used to extract token from `secrecy::Secret<String>` for Bearer header.
7. **Error handling** — explicit k8s API error messages (409 → AlreadyExists, other → status + body).

## Proof it works

```bash
$ cd cmd/factory && cargo check
# Compiles clean
```

## Known issues

- **`--kubeconfig` flag doesn't work** — kube 0.93's public API doesn't support custom kubeconfig paths. Users must use `KUBECONFIG` env var.
- **Auth is hardcoded to Bearer** — client cert auth is not yet supported. The CLI assumes token-based auth.
- **Controller YAML is a placeholder** — `factory install` deploys a placeholder controller. Real controller comes from ticket 04.
- **No `factory status` command** — the CLI suggests `factory status <name>` but it doesn't exist yet.

## Devil's advocate

**Is this ticket actually done?** Mostly. The CLI compiles, has `submit` and `install` subcommands, and handles interactive prompts. But:

- **No `factory status` command** — the CLI tells users to run it, but it doesn't exist.
- **`--kubeconfig` is broken** — returns a helpful error, but it's not actually functional.
- **Controller YAML is placeholder** — `factory install` deploys a non-functional controller.
- **No tests** — no unit or integration tests for the CLI.
- **Auth is incomplete** — only token auth works; client cert and exec auth are stubbed.

## Blocking edges — ticket 04

**Next ticket:** [04 — Factory controller (Go)](/docs/04-factory-controller.md)

**What 04 depends on from this ticket:**
- `deploy/crd.yaml` must be applied before controller starts
- `deploy/k8s/controller.yaml` must exist (placeholder)
- Go types from ticket 02 must be importable

**What 04 will add:**
- `cmd/factory-controller/src/main.go` — k8s controller for Factory CR
- `Makefile` target for building controller
- Update `deploy/k8s/controller.yaml` with real controller image
