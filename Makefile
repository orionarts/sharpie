UI = ./ui/app-window.slint
STYLE ?= fluent

# `preview` needs a slint-viewer built against the same Slint version the
# project links, or newer `.slint` syntax will not parse.  Instead of
# trusting whatever is on PATH, a version-matched viewer is installed under
# target/ and keyed by the locked Slint version, so it is only (re)installed
# when that version changes. Each locked version gets its own install root,
# so switching lockfiles (cargo update, git checkout) always runs the viewer
# for the current lock
SLINT_VERSION     := $(shell grep -A1 '^name = "slint"$$' Cargo.lock | sed -n 's/^version = "\([[:digit:].]*\)"$$/\1/p')
SLINT_VIEWER_ROOT := target/slint-viewer-$(SLINT_VERSION)
# Set SLINT_VIEWER to override the binary.
SLINT_VIEWER      ?= $(SLINT_VIEWER_ROOT)/bin/slint-viewer

build:
	cargo build

gui:
	cargo run

live-preview:
	SLINT_LIVE_PREVIEW=1 cargo run --features slint/live-preview

$(SLINT_VIEWER):
	mkdir -p target
	cargo install slint-viewer --version $(SLINT_VERSION) --root $(SLINT_VIEWER_ROOT)

preview: $(SLINT_VIEWER)
	$(SLINT_VIEWER) --style $(STYLE) $(UI)

docs:
	cargo doc

view-docs:
	cargo doc --open
