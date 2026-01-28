BINDIR = /usr/bin
SYSTEMD_SYSTEM_DIR = /lib/systemd/system

BINARY_NAME = krill-kounter-rs
SERVICE_FILE = $(BINARY_NAME).service

.PHONY: build install uninstall

build:
	cargo build --release

install:
	install -D -m 755 target/release/$(BINARY_NAME) $(DESTDIR)$(BINDIR)/$(BINARY_NAME)
	install -D -m 644 install/service/$(SERVICE_FILE) $(DESTDIR)$(SYSTEMD_SYSTEM_DIR)/$(SERVICE_FILE)
	systemctl daemon-reload
	@echo "Installed system service. Enable with:"
	@echo "  sudo systemctl enable $(SERVICE_FILE)"

uninstall:
	systemctl stop $(SERVICE_FILE) || true
	systemctl disable $(SERVICE_FILE) || true
	rm -f $(BINDIR)/$(BINARY_NAME)
	rm -f $(SYSTEMD_SYSTEM_DIR)/$(SERVICE_FILE)
	systemctl daemon-reload
