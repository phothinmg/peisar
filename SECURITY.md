# Security Policy

## Supported versions

Only the latest release series receives security fixes.

| Version         | Supported |
| --------------- | --------- |
| 1.2.x (latest)  | ✅ Yes    |
| 1.1.x and older | ❌ No     |

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