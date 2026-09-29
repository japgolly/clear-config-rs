demo:
	cargo fmt
	DEBUG=true VERBOSE=No cargo run --example demo
	VERBOSE=maybe cargo run --example demo

t: test
test:
	cargo test --lib
