# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). The repository has no version tags yet.

## [Unreleased]

### Added

- 2026-11-03: Engineering baseline: `SECURITY.md`, `CHANGELOG.md`, PR and issue templates, `.editorconfig`, `.gitattributes`, markdownlint config, pre-commit config, `docs/adr/` with ADR-0001, `make setup`, `make lint` and `make ci`.
- 2026-11-03: CI runs a docs lint job and `make lint` (cargo check and clippy) before the build and tests.

### Changed

- 2026-11-03: `CONTRIBUTING.md` rewritten around `make ci`, branch prefixes and Conventional Commit PR titles.
- 2026-11-03: `rust-toolchain.toml` adds the `rustfmt` and `clippy` components.
- 2026-11-03: CI workflow gets `permissions: contents: read`, `concurrency` and job timeouts. Dependabot is now weekly (cargo) and monthly (Actions), grouped, one open PR each, 7-day cooldown.
