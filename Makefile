demo:
	cargo fmt --all
	DEBUG=true VERBOSE=No ENV=dev SERVER_PORT=3000 cargo run --example demo-explicit
	@echo
	ENV=local VERBOSE=maybe cargo run --example demo-derived

t: test
test:
	cargo fmt --all
	cargo test -q
