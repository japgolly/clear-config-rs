demo:
	cargo fmt --all
	DEBUG=true TIMEOUT=1min ENV=dev SERVER_PORT=3000 cargo run --example demo-explicit
	@echo
	DEBUG=no cargo run --example demo-env_file
	@echo
	ENV=local TIMEOUT=what cargo run --example demo-derived

t: test
test:
	cargo fmt --all
	cargo test -q
