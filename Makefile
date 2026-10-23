.PHONY: build test crd docker clean

# Build everything
build: crd
	cargo build --release
	go build ./cmd/...

# Run tests
test:
	cargo test
	go test ./...

# Generate CRD manifests
crd:
	@echo "CRD manifests in deploy/crd.yaml"

# Build worker Docker image
docker:
	docker build -t factory-worker:latest -f worker/Dockerfile .

# Clean build artifacts
clean:
	cargo clean
	go clean -cache
