default: build

all: test

test: build
	cargo test

build:
	cargo build --release --target wasm32v1-none
	@ls -l target/wasm32v1-none/release/*.wasm

# Compute SHA256 checksum for reproducible WASM builds
checksum:
	@echo "Computing SHA256 for release WASM artifact(s)..."
	@for f in target/wasm32v1-none/release/*.wasm; do \
	  sha256sum "$f" | tee "$f.sha256"; \
	done

# Build and compute checksum in one step
build-checksum: build checksum

fmt:
	cargo fmt --all

clean:
	cargo clean
