demo:
	cargo fmt --all
	DEBUG=true VERBOSE=No ENV=dev PORT=3000 cargo run --example demo
	ENV=local VERBOSE=maybe cargo run --example demo

t: test
test:
	cargo fmt --all
	cargo test --lib
