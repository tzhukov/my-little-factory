.PHONY: build test crd docker controller clean

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

# Generate DeepCopy methods
controller-gen:
	controller-gen object:headerFile= paths=./api/...

# Build controller binary
controller: crd
	go build -o bin/factory-controller ./cmd/factory-controller/...

# Build worker Docker image
docker:
	docker build -t factory-worker:latest -f worker/Dockerfile .

# Clean build artifacts
clean:
	cargo clean
	go clean -cache
