# Security Policy

## Supported versions

Only the latest release receives fixes. This is a fork of
[azerpas/bourso-api](https://github.com/azerpas/bourso-api); issues in code inherited
from upstream are best reported there as well, so every fork benefits.

## Reporting a vulnerability

Report vulnerabilities privately through
[GitHub's private vulnerability reporting](https://github.com/nyckosleducmanage/bourso-api/security/advisories/new),
not through a public issue.

Please include what the issue allows an attacker to do, the steps to reproduce it, and
the version or commit you tested. Expect a first reply within two weeks.

## Scope

This tool drives a BoursoBank account by replaying the web interface, so the sensitive
surface is credential handling and session management:

- the customer id is stored in `~/.bourso/settings.json`
- the password is prompted at each run and is never written to disk, unless a credentials
  file is deliberately passed with `--credentials`, in which case protecting that file is
  the operator's responsibility
- `~/.bourso/bourso.log` records HTTP exchanges at debug level and can contain account
  identifiers and session material

Anything that leaks these beyond the local machine, or that could send a request to an
account other than the authenticated one, is in scope.

Out of scope: BoursoBank's own website and infrastructure. Report those to BoursoBank.
