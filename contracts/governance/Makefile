default: build

all: test

test: build
	cargo test

build:
	cargo build --release --target wasm32v1-none
	@ls -l ../../target/wasm32v1-none/release/governance_events.wasm 2>/dev/null || \
	  ls -l target/wasm32v1-none/release/governance_events.wasm 2>/dev/null || true

# Check that the governance-events WASM stays within its 24 KB size budget.
#
# This mirrors the CI gate in .github/workflows/wasm-size.yml.
# Run locally with: make check-size
check-size: build
	@WASM=$$(find ../../target/wasm32v1-none/release target/wasm32v1-none/release \
	           -name 'governance_events.wasm' 2>/dev/null | head -1); \
	if [ -z "$$WASM" ]; then \
	  echo "governance_events.wasm not found — run 'make build' first"; exit 1; \
	fi; \
	SIZE=$$(stat -f%z "$$WASM" 2>/dev/null || stat -c%s "$$WASM"); \
	BUDGET=24576; \
	SIZE_KB=$$(echo "scale=1; $$SIZE / 1024" | bc); \
	BUDGET_KB=$$(( $$BUDGET / 1024 )); \
	echo "governance_events.wasm: $${SIZE_KB} KB / $${BUDGET_KB} KB budget"; \
	if [ "$$SIZE" -gt "$$BUDGET" ]; then \
	  echo "ERROR: size budget exceeded ($$SIZE bytes > $$BUDGET bytes)"; exit 1; \
	else \
	  echo "OK: within budget"; \
	fi

fmt:
	cargo fmt --all

clean:
	cargo clean
