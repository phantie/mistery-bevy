run:
	RUST_LOG="mistery=debug" cargo run --bin mistery

t:
	cargo run --bin testing

test:
	cargo test

tst:
	make test

fmt:
	cargo fmt

ftm:
	make fmt