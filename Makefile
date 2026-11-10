BIN := vm-tray
PREFIX ?= $(HOME)/.local
UNIT_DIR := $(HOME)/.config/systemd/user

.PHONY: build run install clean

build:
	cargo build --release

run: build
	./target/release/$(BIN)

install: build
	install -Dm755 target/release/$(BIN) $(PREFIX)/bin/$(BIN)
	install -Dm644 data/$(BIN)-symbolic.svg $(PREFIX)/share/icons/hicolor/scalable/apps/$(BIN)-symbolic.svg
	mkdir -p $(UNIT_DIR)
	sed 's|%h/.local|$(PREFIX)|' $(BIN).service > $(UNIT_DIR)/$(BIN).service
	systemctl --user daemon-reload
	systemctl --user enable $(BIN).service
	systemctl --user restart $(BIN).service

clean:
	cargo clean
