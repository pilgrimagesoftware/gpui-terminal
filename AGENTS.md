# AGENTS.md

`gpui-terminal` is a standalone, MIT-licensed Rust crate: a terminal emulator
view for GPUI, backed by `alacritty_terminal`, over any byte transport. It is
embedded by [Knot](https://github.com/pilgrimagesoftware/Knot) and
[Fernrohr](https://github.com/pilgrimagesoftware/Fernrohr) - see `README.md`
for the public API.

## Build and test

```sh
cargo build --all-features
cargo test --all-features
cargo test --no-default-features
cargo clippy --all-targets --all-features -- -D warnings
cargo clippy --no-default-features --all-targets -- -D warnings
```

Formatting uses a pinned nightly rustfmt (`rustfmt.toml` sets
`unstable_features = true` for struct-field alignment, which only a nightly
rustfmt honors):

```sh
rustup toolchain install nightly-2026-09-21 --profile minimal --component rustfmt
rustup run nightly-2026-09-21 cargo fmt --all --check
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy (both feature sets) and test
(both feature sets, on macOS and Linux) on every push to `main` and every pull
request.

## Conventions

- **Conventional Commits**, signed (`git commit -S` or a configured signing
  key) - never bypass signing.
- No `[patch]` sections and no git dependencies. Everything resolves from
  crates.io.
- The `gpui-kit = "=0.7.0"` pin is exact and must be bumped in lockstep with
  every consumer (Knot, Fernrohr) in the same change - two `gpui` versions in
  one build are two incompatible `Entity` types.
- Files stay under ~500 lines; split a module before it gets there rather than
  after.
- MIT license. This crate must never depend on AGPL (or otherwise
  copyleft-incompatible) code - that is the reason it is a separate repo from
  Knot rather than a path dependency.
