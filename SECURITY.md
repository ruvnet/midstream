# Security Policy

## Reporting a vulnerability

**Do not open a public GitHub issue for security reports.** Use one of
the channels below.

### Preferred: GitHub Security Advisories

File a private advisory at
<https://github.com/ruvnet/midstream/security/advisories/new>. This
opens a private conversation with the maintainers, scoped to the
issue, with built-in CVE coordination.

### Email fallback

If you can't use GitHub Security Advisories, email
**security@ruv.net** with:

- A description of the vulnerability and its impact.
- Reproduction steps (or a proof-of-concept).
- The affected crate name and version (or commit SHA).
- Your suggested fix, if any.

PGP-encrypted reports are accepted; key fingerprint will be published
here once generated. For now, plain email is fine.

## Response timeline

| Step | Target |
|---|---|
| Acknowledgement of receipt | within **72 hours** |
| Initial assessment and triage | within **7 days** |
| Fix in a release candidate | within **30 days** for high-severity |
| Public disclosure / advisory | **90 days** from initial report, or coordinated earlier if a patch is available |

If we miss any of these targets, we will say so in
`docs/triage-log.md` and the corresponding advisory thread. Misses are
public; we don't hide them.

## Supported versions

Per [ADR-0024](docs/adr/0024-semver-and-api-stability.md), each crate
has a stability tier. Security backports are provided as follows:

| Tier | Versions receiving backports |
|---|---|
| **stable** (1.0+) | latest minor + previous minor |
| **beta** | latest released version only |
| **alpha** | latest released version only (best effort) |

All current crates are alpha or beta. The full table lives in the ADR.

## What counts as a vulnerability

- Memory safety bugs (use-after-free, out-of-bounds, data race) in
  first-party code.
- Cryptographic misuse (e.g. accepting invalid certificates by default
  — see ADR-0011).
- Authentication, authorization, or input-validation bypasses.
- Denial-of-service vectors that an attacker can trigger with bounded
  input (e.g. unbounded allocation from network input —
  ADR-0012/0015).
- Supply-chain issues we can act on (e.g. a banned/vulnerable
  transitive — ADR-0014 / `deny.toml`).

## Not in scope (please don't file as security)

- Performance regressions in benchmark code.
- Bugs that require attacker-controlled local filesystem or unrestricted
  shell access to exploit.
- Issues in the vendored `hyprstream-main/` directory while ADR-0002
  is in flight — those should be reported upstream.

## Public disclosure

Once a patch is available we publish a GitHub Security Advisory,
request a CVE, and yank affected versions from crates.io. The advisory
credits the reporter unless they request anonymity.
