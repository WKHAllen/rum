all: build

build:
	cargo build

run:
	cargo run

test:
	cargo test -- --nocapture

coverage:
	cargo llvm-cov --open

lint:
	cargo clippy -- -D warnings

clean:
	cargo clean
