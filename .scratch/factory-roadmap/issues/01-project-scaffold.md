# 01 — Project scaffold: go.mod + Cargo workspace + Makefile

**What to build:** Go module, Rust workspace, and build infrastructure. The project compiles with `make build`.

**Blocked by:** None — can start immediately.

**Status:** in-progress

- [ ] `go.mod` initialized at root with module path `github.com/earendil-works/factory`
- [ ] `Cargo.toml` workspace with members `cmd/factory` and `worker`
- [ ] `rust-toolchain.toml` pinning the Rust version
- [ ] `Makefile` with targets: `build`, `test`, `crd`, `docker`
- [ ] `make build` compiles both Go and Rust components
- [ ] Directory structure from spec: `api/v1/`, `pkg/factory/`, `pkg/model/`, `pkg/k8s/`, `cmd/factory/`, `worker/`, `deploy/`, `test/e2e/`
