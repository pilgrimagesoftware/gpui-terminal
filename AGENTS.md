# AGENTS.md

## About This Project

`gpui-terminal` is a standalone, MIT-licensed Rust crate: a terminal emulator
view for GPUI, backed by `alacritty_terminal`, over any byte transport. It is
embedded by [Knot](https://github.com/pilgrimagesoftware/Knot) and
[Fernrohr](https://github.com/pilgrimagesoftware/Fernrohr) - see `README.md`
for the public API.

Both consumers take it as a git dependency pinned to a tag
(`gpui-terminal = { git = "...", tag = "vX.Y.Z" }`). Knot runs a local PTY
(default `pty` feature); Fernrohr turns `pty` off and implements `Transport`
over a Kubernetes exec session. A change only reaches them through a new tag.

## Dependency relationships

- `alacritty_terminal` - VT/ANSI parsing and the cell grid (`Grid`).
- `gpui-kit` (core GPUI only, `default-features = false`) - `TerminalView`.
  Pinned exactly; see Conventions.
- `portable-pty` - `PtyTransport`, only with the `pty` feature.
- `async-channel` - wakes the view when a transport thread delivers output.

## Running Checks Locally

```sh
make             # fmt-check + lint + test + build, both feature sets

make fmt         # reformat with the pinned nightly
make fmt-check   # verify formatting (what CI runs)
make lint        # clippy -D warnings, pty on and off
make test        # cargo test, pty on and off
make build
```

Formatting uses a pinned nightly rustfmt (`rustfmt.toml` sets
`unstable_features = true` for struct-field alignment, which only a nightly
rustfmt honors). The pin is `RUSTFMT_NIGHTLY` in the `Makefile` and in
`.github/workflows/ci.yml` - keep them equal:

```sh
rustup toolchain install $(make -s print-rustfmt-nightly) --profile minimal --component rustfmt
```

CI (`.github/workflows/ci.yml`) runs fmt, clippy (both feature sets) and test
(both feature sets, on macOS and Linux) on every push to `master` or `develop`
and every pull request. The `master` and `develop` rulesets both require
`fmt`, `clippy`, `test (macos-latest)` and `test (ubuntu-latest)` - renaming a
job means updating both rulesets too.

## Committing Code

[Conventional Commits](https://www.conventionalcommits.org/), signed
(`git commit -S` or a configured signing key) - never bypass signing. Scope is
the module touched:

```
feat(view): scroll the scrollback with the wheel
fix(grid): keep the selection anchored across a resize
build(deps): bump alacritty_terminal to 0.27
```

## Branches and Workflow

- git-flow. `develop` is the integration branch and the default; `master` is
  release-only. Both are protected: changes land through a PR with green CI
  and signed commits; no force-push, no deletion.
- Branch from `develop` as `feat/<change>`, `fix/<change>` or
  `chore/<change>`, and PR back to `develop`. Only `release/x.y.z` and
  `hotfix/x.y.z` PR to `master`. Never commit straight to either. Do the work
  in a `git worktree` in the peer directory `<checkout>-wt/<change>`, never in
  the primary checkout.
- Merge with a merge commit or rebase, never squash - the Conventional Commit
  prefixes are the changelog's raw material.
- Releases are signed tags on `master` after a `release/x.y.z` PR (version +
  changelog bump), followed by a `master` -> `develop` merge-back; see
  `CONTRIBUTING.md` - Releases. Consumers then bump their `tag` pin.

## Conventions

- No `[patch]` sections and no git dependencies. Everything resolves from
  crates.io - a consumer's build cannot apply a patch made here.
- The `gpui-kit = "=0.7.0"` pin is exact and must be bumped in lockstep with
  every consumer (Knot, Fernrohr) in the same change - two `gpui` versions in
  one build are two incompatible `Entity` types. Dependabot ignores it.
- Files stay under ~500 lines; split a module before it gets there rather than
  after.
- MIT license. This crate must never depend on AGPL (or otherwise
  copyleft-incompatible) code - that is the reason it is a separate repo from
  Knot rather than a path dependency.
- User-facing changes get a line under `## [Unreleased]` in `CHANGELOG.md`.

## Architecture Decisions

Non-trivial design choices are recorded as ADRs under `docs/adr/`. Index and
process: `docs/adr/README.md`. `/adr "<title>"` scaffolds a new record from
`docs/adr/0000-template.md`.
