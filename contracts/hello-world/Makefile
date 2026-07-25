default: build

all: test

test: build
	cargo test

build:
	cargo build --release --target wasm32v1-none
	@ls -l target/wasm32v1-none/release/*.wasm

fmt:
	cargo fmt --all

clean:
	cargo clean
