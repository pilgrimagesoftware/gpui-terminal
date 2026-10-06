# Security Policy

## Reporting a vulnerability

Report privately through GitHub: open the repository's **Security** tab and use
**Report a vulnerability** (GitHub Security Advisories). Do not file a public
issue for a security problem.

A terminal emulator parses bytes from programs it does not control, so issues
in escape-sequence handling, paste encoding (bracketed-paste escapes) or
clipboard requests (OSC 52) are in scope.

Include what you can:

- affected version / tag or commit
- steps to reproduce or a proof of concept (e.g. the byte sequence)
- impact assessment

You will get an acknowledgement within a few days. Once a fix is ready a
coordinated disclosure date will be agreed with you.

## Supported versions

The crate is pre-1.0. Only the latest tag on `main` receives security fixes.
