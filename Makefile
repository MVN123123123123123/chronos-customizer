.PHONY: all test build-host build-android package-module verify-16kb clean

NDK_TARGET ?= aarch64-linux-android
TARGET_DIR ?= target/$(NDK_TARGET)/release
MODULE_DIR ?= chronos-module
DIST_DIR ?= dist

all: build-host test

test:
	cargo test --workspace

build-host:
	cargo build --workspace --release

build-android:
	@echo "Cross-compiling Rust crates for Android ($(NDK_TARGET)) with 16KB ELF page size..."
	cargo ndk -t arm64-v8a build --release --workspace

package-module: build-android
	@echo "Packaging unified companion module for Magisk, KernelSU, and APatch..."
	@mkdir -p $(MODULE_DIR)/bin
	@mkdir -p $(MODULE_DIR)/zygisk
	@mkdir -p $(DIST_DIR)
	@cp $(TARGET_DIR)/customd $(MODULE_DIR)/bin/
	@cp $(TARGET_DIR)/libchronos_zygisk.so $(MODULE_DIR)/zygisk/arm64-v8a.so
	@cd $(MODULE_DIR) && zip -r ../$(DIST_DIR)/chronos-companion-module.zip . -x "*.git*"
	@echo "Successfully packaged: $(DIST_DIR)/chronos-companion-module.zip"

verify-16kb:
	@echo "Verifying 16KB ELF page size alignment for Android 15-17 compatibility..."
	@if [ -f "$(TARGET_DIR)/libchronos_zygisk.so" ]; then \
		readelf -W -l $(TARGET_DIR)/libchronos_zygisk.so | grep -E "LOAD"; \
	else \
		echo "Run 'make build-android' first."; \
	fi

clean:
	cargo clean
	rm -rf $(DIST_DIR)
	rm -rf $(MODULE_DIR)/bin $(MODULE_DIR)/zygisk
