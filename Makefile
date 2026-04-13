BINARY     := preman
INSTALL_DIR ?= $(HOME)/.local/bin
TARGET_DIR := target/release

.PHONY: all build debug run release install uninstall clean lint fmt check test help

## Default: build release binary
all: release

## Build debug binary (fast compile, includes debug symbols)
build:
	cargo build

## Build optimised release binary
release:
	cargo build --release

## Run in debug mode
run:
	cargo run

## Run release binary directly
run-release: release
	./$(TARGET_DIR)/$(BINARY)

## Install release binary to INSTALL_DIR (default: ~/.local/bin)
install: release
	@mkdir -p $(INSTALL_DIR)
	@cp $(TARGET_DIR)/$(BINARY) $(INSTALL_DIR)/$(BINARY)
	@chmod +x $(INSTALL_DIR)/$(BINARY)
	@echo "Installed $(BINARY) to $(INSTALL_DIR)"
	@if ! echo "$$PATH" | grep -q "$(INSTALL_DIR)"; then \
		echo "Note: $(INSTALL_DIR) is not in PATH. Add it with:"; \
		echo "  export PATH=\"\$$PATH:$(INSTALL_DIR)\""; \
	fi

## Uninstall binary from INSTALL_DIR
uninstall:
	@rm -f $(INSTALL_DIR)/$(BINARY)
	@echo "Removed $(INSTALL_DIR)/$(BINARY)"

## Run clippy linter
lint:
	cargo clippy -- -D warnings

## Format source code
fmt:
	cargo fmt

## Check formatting without modifying files
fmt-check:
	cargo fmt -- --check

## Run all checks (format + lint + build + test)
check: fmt-check lint build test

## Run tests
test:
	cargo test

## Remove build artifacts
clean:
	cargo clean

## Show binary size breakdown (requires cargo-bloat: cargo install cargo-bloat)
bloat:
	cargo bloat --release --crates

## Print help
help:
	@echo "Usage: make [target]"
	@echo ""
	@awk '/^## /{desc=$$0; next} /^[a-zA-Z_-]+:/{gsub(/## /,"",desc); printf "  \033[36m%-16s\033[0m %s\n", $$1, desc; desc=""}' $(MAKEFILE_LIST)
