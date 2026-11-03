# Contributing to fundus

`main` stays stable. Work happens on branches and lands through a pull request. Run `make ci` before you push: it runs cargo check, clippy, the release build, the in-kernel tests and the QEMU shell test.

## Setup

Needs `rustup`, QEMU (`qemu-system-x86_64`) and Python 3.

```sh
make setup   # installs the nightly pinned in rust-toolchain.toml and bootimage
make help    # list targets
```

## Branches and pull requests

| Prefix | Use |
| :---- | :---- |
| `feat/...` | New kernel features |
| `fix/...` | Bug fixes |
| `docs/...` | Documentation |
| `chore/...` | Tooling and repository hygiene |
| `ci/...` | Workflow changes |
| `test/...` | Tests only |

- PR titles follow Conventional Commits, matching the prefix: `feat: add ...`, `fix: ...`.
- One purpose per PR. Fill in `.github/PULL_REQUEST_TEMPLATE.md`.
- Commit messages state what changed and why. Weak: `updated files`. Strong: `fix: clamp heap stats when the free list is empty`.

## Running tests and lint

```sh
make lint         # cargo check + cargo clippy (rustfmt differences are advisory)
make test-kernel  # in-kernel #[test_case] suite, boots QEMU (cargo test)
make test         # builds the release image, boots it headless, drives the shell
make ci           # all of the above plus the build
```

## Code style

- Format new code with `cargo fmt`; existing files predate the formatter and are not mass-reformatted.
- Fix clippy warnings in the code you touch.
- Every `unsafe` block needs a `// SAFETY:` comment saying why it is sound.
- The kernel has a heap allocator, so heap allocation is fine. Keep the kernel minimal.
- Document design decisions in comments or in `docs/adr/`.

## Reporting issues

Use the issue templates. For bugs, include the QEMU version, the Rust toolchain (`rustc --version`) and the serial output. See [TROUBLESHOOTING.md](TROUBLESHOOTING.md) first, and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for design background. Report vulnerabilities as described in [SECURITY.md](SECURITY.md).

## License

By contributing, you agree that your contributions are licensed under the MIT License.
