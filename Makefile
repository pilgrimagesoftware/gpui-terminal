# The same checks CI runs (`.github/workflows/ci.yml`), in the same order.
# Both feature sets are checked: `pty` is on by default, and an embedder that
# brings its own transport builds with it off.

# `rustfmt.toml` turns on `unstable_features`, which only a nightly rustfmt
# honors, so formatting is reproducible on one exact nightly. Keep in step
# with `RUSTFMT_NIGHTLY` in ci.yml. Install with
# `rustup toolchain install $(make -s print-rustfmt-nightly) --component rustfmt`.
RUSTFMT_NIGHTLY := nightly-2026-09-21

.PHONY: all fmt fmt-check lint test build print-rustfmt-nightly

all: fmt-check lint test build

fmt:
	rustup run $(RUSTFMT_NIGHTLY) cargo fmt --all

fmt-check:
	rustup run $(RUSTFMT_NIGHTLY) cargo fmt --all --check

lint:
	cargo clippy --all-targets --all-features -- -D warnings
	cargo clippy --no-default-features --all-targets -- -D warnings

test:
	cargo test --all-features
	cargo test --no-default-features

build:
	cargo build --all-features
	cargo build --no-default-features

print-rustfmt-nightly:
	@echo $(RUSTFMT_NIGHTLY)
