TARGET = x86_64-unknown-linux-musl

.PHONY: build static setup

build:
	cargo build --release

static: setup
	CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc \
	cargo build --target $(TARGET) --release
	@echo ""
	@echo "Binary: target/$(TARGET)/release/tudiff"
	@ldd target/$(TARGET)/release/tudiff 2>&1 || true

setup:
	rustup target add $(TARGET)
	@which x86_64-linux-musl-gcc > /dev/null 2>&1 || \
		(echo "Installing musl-tools..." && sudo apt-get install -y musl-tools)
