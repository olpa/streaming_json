.PHONY: build test install

all:


fix:
	cargo fmt

lint:
	cargo check
	cargo clippy -- -W clippy::pedantic -W clippy::panic
