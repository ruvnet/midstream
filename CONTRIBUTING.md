# Contributing to midstream

Thanks for considering a contribution. This document is short on
purpose: read it once, then refer back when something specific comes
up.

## Before you start

1. Read the **[Architecture Decision Records](docs/adr/README.md)** —
   they're the canonical answer to *why* the project looks the way it
   does. Most surprising design choices are captured there. If you
   want to challenge a decision, write a superseding ADR rather than
   working around it.
2. Skim the [Code of Conduct](CODE_OF_CONDUCT.md). It's the
   Contributor Covenant — short.
3. Security issues: see [SECURITY.md](SECURITY.md). **Do not open a
   public issue for a security report.**

## Getting set up

```bash
git clone https://github.com/ruvnet/midstream.git
cd midstream

# Verify your toolchain matches the project MSRV (1.81 per ADR-0023):
rustc --version
```

Build / test:

```bash
# All 6 publishable workspace crates (the only fully-buildable subset
# today; ADR-0002 will fix the root midstream crate once the vendored
# hyprstream-main is removed).
cargo check --workspace --exclude midstream --exclude hyprstream
cargo test  --workspace --exclude midstream --exclude hyprstream --lib
```

## Sending a PR

1. **Fork + branch.** Branch names use `<type>/<short-desc>-adr<NNNN>`
   when implementing a specific ADR, e.g. `feat/bytes-hot-path-adr0006`.
2. **One concern per PR.** Mechanical refactors, security fixes, and
   feature work each get their own PR.
3. **Commit messages** follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` / `fix:` / `chore:` / `docs:` / `refactor:` / `perf:` / `test:` / `build:` / `ci:`
   - Append `!` for breaking changes (e.g. `feat!:` or `fix(quic)!:`).
   - The body explains *why*; the diff shows *what*.
4. **Every commit must include a `Signed-off-by:` trailer.** This is
   the [Developer Certificate of Origin](https://developercertificate.org/)
   sign-off — there is no CLA. Set it up once:

   ```bash
   git config --global user.name  "Your Name"
   git config --global user.email "you@example.com"
   # Then commit with -s:
   git commit -s -m "feat: …"
   ```
5. **Link the ADR.** If your PR implements an ADR, cite it in the PR
   description and in commit messages. If your PR changes a public API
   or crosses a module boundary, *write* an ADR first.
6. **CI must pass.** The audit + MSRV + test matrix must all be green
   before merge. Use `cargo xtask ci` (once ADR-0037 lands) to
   reproduce the same gates locally.

## What we care about in review

- **Correctness over cleverness.** Boring code is fine.
- **Bench / measure when claiming perf.** ADR-0009 sets the bar:
  numbers must come from criterion runs, not hand-typed prose.
- **No silent re-licensing.** Code carries `MIT OR Apache-2.0` per
  ADR-0036.
- **No unwrap / expect in production code paths** (per ADR-0018).
  Tests, examples, and benches are fine.
- **Reverse comments to *why*, not *what*.** Well-named identifiers
  cover the what.

## Reporting bugs

Use the bug report issue template. Include:

- The exact crate version (and git rev if applicable).
- Toolchain (`rustc --version`).
- Minimal reproducer.
- Expected vs actual behaviour.

## Asking questions

For project-direction discussions, file an issue with the "discussion"
label or write a draft ADR. We don't currently have a chat channel.

## License

By contributing you agree your contribution is dual-licensed under MIT
or Apache-2.0 at the user's choice (see [NOTICE](NOTICE)). The DCO
sign-off makes this explicit on every commit.
