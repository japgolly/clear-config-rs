demo:
	cargo fmt --all
	DEBUG=true TIMEOUT=1min ENV=dev SERVER_PORT=3000 cargo run --example demo-explicit
	@echo
	DEBUG=no cargo run --example demo-env_file
	@echo
	API_KEY=123 ENV=local SERVER_HOST= TIMEOUT=what cargo run --example demo-derived

t: test
test:
	cargo fmt --all
	cargo test -q
