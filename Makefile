APP := war-dodger

.PHONY: check test bench release verify install clean
check:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings
test:
	cargo test
bench:
	cargo bench
release:
	cargo build --release
verify: check test bench release
install: release
	install -Dm755 target/release/$(APP) $(PREFIX)/bin/$(APP)
clean:
	cargo clean
