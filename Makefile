.PHONY: test-native test-wasm build-native build-wasm build-wasm-tests check-wasm help

help:
	@echo "Synapse Makefile"
	@echo "  make build-native    - Build for native (x86_64-unknown-linux-gnu)"
	@echo "  make build-wasm      - Build for wasm32-wasip1"
	@echo "  make build-wasm-tests - Build wasm tests (verify wasm target)"
	@echo "  make test-native     - Run native tests"
	@echo "  make test-wasm       - Verify wasm build (no runtime)"
	@echo "  make check-wasm      - Build wasm and run smoke test"

build-native:
	cargo build --target x86_64-unknown-linux-gnu

build-wasm:
	cargo build --target wasm32-wasip1

build-wasm-tests:
	@echo "Building wasm lib only (tests depend on native-only deps like mockito)"
	cargo build --target wasm32-wasip1 --lib

test-native:
	cargo test --target x86_64-unknown-linux-gnu

test-wasm:
	@echo "Wasm runtime tests require wasmtime + PFC host functions."
	@echo "See docs/PFC_INTEGRATION.md for full e2e test setup."
	@echo "Verifying wasm build only..."
	cargo build --target wasm32-wasip1 --lib
	@echo "Wasm build OK."

check-wasm: build-wasm test-wasm
	@echo "Wasm check complete."
