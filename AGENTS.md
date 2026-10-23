# AGENTS.md

## DevOps Agent Instructions

### Ticket Workflow

Every ticket must go through this cycle:

1. **Devil's Advocate** — Before starting, challenge the ticket:
   - Are the acceptance criteria actually the right criteria?
   - Is the scope right? Too coarse? Too fine?
   - What assumptions am I making?
   - What could go wrong?

2. **Implement** — Build only what's required. Nothing more.

3. **Verification** — Prove it works:
   - `go test ./...` passes
   - `cargo test` passes
   - Manual smoke test against Kind or bare-metal k8s
   - No regressions in existing functionality

4. **Devil's Advocate (Post)** — After implementation, challenge the work:
   - Is the code actually done, or is it scaffolding that looks done?
   - Did I add more than was asked?
   - Are there hidden assumptions or broken edges?
   - What would break if someone ran this right now?

5. **Document** — Create `docs/<ticket-number>-<slug>.md` with:
   - **What was done** — concise, factual
   - **Decisions made** — trade-offs, alternatives considered
   - **Proof it works** — commands run, outputs, test results
   - **Known issues** — what's broken or incomplete

6. **Commit** — Squash into a single commit. Update README checkbox.

### Code Standards

- **Go**: `go vet ./...` must pass. `gofmt` formatting. No bare `error` returns — wrap with context.
- **Rust**: `cargo clippy` must pass. `cargo fmt` formatting. No `unwrap()` in production code.
- **CRD types**: YAML and Go types must stay in sync. Document every field.
- **K8s manifests**: No hardcoded values. Use kustomization overlays.

### Branch Strategy

- `main` is the working branch. Fast iteration.
- PRs when the project has enough commits to warrant review (~5+ meaningful commits).
- Commit messages: `<ticket> — <description>`

### Testing

- Unit tests at the seam (public interfaces only)
- E2E tests against Kind cluster
- No integration tests against bare-metal k8s until the project is stable
