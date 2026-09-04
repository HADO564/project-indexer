# Security Policy

## Supported versions

Project Indexer is pre-1.0. Only the latest release receives fixes; there are no
backports to older tags.

| Version | Supported |
|---|---|
| 0.1.x (latest) | ✅ |
| anything older | ❌ |

## Reporting a vulnerability

**Please do not open a public issue for a security problem.**

Use GitHub's private reporting instead: go to the repository's **Security** tab
and choose **Report a vulnerability**. That opens a draft advisory visible only
to you and the maintainers.

Useful things to include, as far as you can establish them:

- what an attacker can achieve, not just what misbehaves
- the version, operating system, and how the app was installed
- steps to reproduce, ideally from a clean profile
- whether it needs local access, a malicious project directory, or neither

You can expect an acknowledgement within a week. If a report turns out to be a
plain bug rather than a vulnerability, it will be moved to a normal issue with
your agreement.

## Scope

Project Indexer is a local desktop app. It has no server, no account system, and
makes no outbound network requests of its own. The security surface is
correspondingly narrow, and reports are most likely to be relevant if they
involve:

- **Untrusted project directories.** The app parses `.uproject` files, reads git
  repository metadata, and walks directories chosen by the user. A repository or
  project file crafted to exploit that parsing is in scope.
- **Application launching.** Projects store an "open with" command line, and on
  Linux that comes from `.desktop` entries. Anything that turns opening a project
  into unintended command execution is in scope.
- **The project database.** `projects.db` is a plain SQLite file in your config
  directory, deliberately readable by other tools (see the cross-app contract in
  `docs/architecture.md`). It is not encrypted, and that is by design — it holds
  paths and labels, not credentials. Reports that amount to "another local
  process can read it" describe intended behaviour.

Out of scope: anything requiring an attacker who already has your user account
on your machine, since at that point they can read the same files directly.
