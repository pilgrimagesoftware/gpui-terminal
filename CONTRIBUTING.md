# Contributing to gpui-terminal

`gpui-terminal` is a terminal emulator view for GPUI, embedded by
[Knot](https://github.com/pilgrimagesoftware/Knot) and
[Fernrohr](https://github.com/pilgrimagesoftware/Fernrohr). Both consume it as
a git dependency pinned to a tag, so every change here reaches them through a
release, never by tracking a branch.

## Before you start

- The Rust toolchain is pinned by `rust-toolchain.toml` (1.98.0, with
  `rustfmt` and `clippy`). `rustup` picks it up automatically.
- Formatting uses a pinned nightly `rustfmt`, named by `RUSTFMT_NIGHTLY` in the
  `Makefile`: `rustup toolchain install $(make -s print-rustfmt-nightly)
  --profile minimal --component rustfmt`.
- On Linux, GPUI needs the system libraries listed in the `apt-get install`
  step of `.github/workflows/ci.yml`.

## Workflow

git-flow. `develop` is the integration branch and the default; `master` is
release-only and only ever receives `release/*` and `hotfix/*` merges.

1. Branch from `develop`: `feat/<change>`, `fix/<change>`, `chore/<change>`.
2. Keep commits scoped, conventional and signed (see below).
3. Open a PR against `develop` (`release/*` and `hotfix/*` PR to `master`). CI (`.github/workflows/ci.yml`) must pass: `fmt`,
   `clippy`, and `test` on macOS and Linux, with the `pty` feature both on and
   off.
4. PRs merge with a merge commit or rebase - never squash - so Conventional
   Commit prefixes survive in history. Delete a merged feature branch at merge
   time (`gh pr merge --merge --delete-branch`). The repo's "Automatically
   delete head branches" setting stays off: it deleted `master` when a
   back-merge PR used it as the head branch.

## Running checks locally

```bash
make             # fmt-check + lint + test + build, both feature sets

make fmt         # reformat with the pinned nightly
make fmt-check   # verify formatting (what CI runs)
make lint        # clippy -D warnings, pty on and off
make test
make build
```

Run `make fmt` before committing.

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/), signed. Scope is
the module touched:

```
feat(view): scroll the scrollback with the wheel
fix(grid): keep the selection anchored across a resize
feat(style): a host palette for the terminal's colours
build(deps): bump alacritty_terminal to 0.27
```

## Dependencies

- Everything resolves from crates.io: no `[patch]` sections, no git
  dependencies. A consumer's build has no way to apply a patch made here.
- `gpui-kit` is pinned exactly and bumped by hand, in the same change as every
  consumer's pin. Dependabot ignores it on purpose.
- MIT only. Nothing copyleft - this crate exists as a separate repo so that
  AGPL code (Knot) never ends up in it.

## Architecture decisions

Non-trivial design choices get an ADR under `docs/adr/`. Run `/adr "<title>"`
or copy `docs/adr/0000-template.md`. Index: `docs/adr/README.md`.

## Changelog

User-facing changes go under `## [Unreleased]` in `CHANGELOG.md` in the
Keep a Changelog format (Added / Changed / Fixed / Removed).

## Releases

Consumers pin a tag, so a release is a signed tag on `master`:

1. Branch `release/x.y.z` from `develop`. Bump `version` in `Cargo.toml` and
   move the `[Unreleased]` entries in `CHANGELOG.md` under
   `## [x.y.z] - <date>`. Open the PR against `master`.
2. After it merges, tag the merge commit on `master`:
   `git tag -s vX.Y.Z -m "vX.Y.Z" && git push origin vX.Y.Z`.
   The rulesets cover branches only, so tag pushes are not gated.
3. Merge `master` back into `develop` through a PR, so the version bump and
   changelog reach the integration branch. `master` is that PR's head branch:
   merge it without `--delete-branch`.
4. Bump the `tag` in each consumer's `Cargo.toml` in its own PR.

A fix that cannot wait for `develop` goes on `hotfix/x.y.z` from `master`,
PRs to `master`, and follows steps 2-4.

Pre-1.0, a breaking API change bumps the minor version. The crate does not
publish to crates.io.

## License

By contributing you agree your work is licensed under the MIT license, matching
the project.
