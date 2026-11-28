BINARY := hyprhyde

.PHONY: build install clean test e2e

build:
	cargo build --release --locked

install: build
	install -Dm755 target/release/$(BINARY) $(HOME)/.local/bin/$(BINARY)

clean:
	cargo clean

test:
	cargo test --locked

e2e:
	sh tests/e2e/run.sh
