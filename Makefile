.PHONY: test build wasm fmt lint deploy clean

test:
	cargo test

lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

wasm:
	stellar contract build

deploy:
	./scripts/deploy.sh

clean:
	cargo clean
