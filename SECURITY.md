# Security Policy

## Supported versions

Only the latest release receives security fixes.

| Version        | Supported   |
| -------------- | ----------- |
| 1.0.x (latest) | ✅ Yes      |
| 0.3.x          | ✅ Yes      |
| 0.2.x and older | ❌ No      |

### About the `PeisarCache` history

The in-memory Markdown/asset cache (`PeisarCache`), the `peisarSsgConfig()`
export, and the `Peisar.toml` loader shipped in npm versions 0.1.3–0.1.5.
They were **removed in 0.2.0**, and the SSG tooling moved to the separate
`peisar-ssg` project. `PeisarCache` itself was subsequently **restored** to
the `peisar` package (see [CHANGELOG.md](CHANGELOG.md)) so `peisar-ssg`
consumes it from `peisar` again. The `peisarSsgConfig()` export and the
`Peisar.toml` loader remain removed and live in `peisar-ssg`.

Because 0.1.x is out of support, any security issue reported against the old
`0.1.x` cache or configuration loader will not be patched in place. The
remediation is to migrate to the latest `peisar` release or to the
`peisar-ssg` package.

## Trust model

Peisar parses Markdown; it **does not sanitize it**. This is standard
CommonMark behavior:

- Raw HTML blocks and inline HTML pass through unescaped.
- Link and image URLs are attribute-escaped but not scheme-filtered
  (`javascript:` and `data:` URLs are not rejected).
- Text content and attribute values are escaped.

Only render Markdown from sources you trust. When rendering untrusted input,
pipe the rendered HTML through a sanitizer (for example DOMPurify) before
inserting it into a page.

## Reporting a vulnerability

Please report privately via
[GitHub Security Advisories](https://github.com/phothinmg/peisar/security/advisories/new):

1. Open the advisory form linked above and select "Report a vulnerability".
2. Include the affected version, a minimal reproduction, and the impact.

Please do not open public issues or pull requests for suspected security
vulnerabilities. Reports are acknowledged within 14 days.