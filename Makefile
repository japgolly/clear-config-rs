demo:
	cargo fmt
	DEBUG=true VERBOSE=false cargo run --example demo
	cargo run --example demo

t: test
test:
	cargo test --lib
