.PHONY: build test install

all:


fix:
	cargo fmt

lint:
	cargo check
	cargo clippy -- -W clippy::pedantic -W clippy::panic -W clippy::unwrap_used -W clippy::expect_used -W clippy::indexing_slicing -W unsafe_code -W missing_docs
