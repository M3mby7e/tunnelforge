# Security Policy

Tunnelium handles SSH credentials and network connections, so security matters.

## Reporting a vulnerability

**Please do not open a public GitHub issue for security problems.**

Instead, report privately via GitHub's **"Report a vulnerability"** (Security → Advisories)
on the repository, or email the maintainer listed on the GitHub profile. Include:

- affected version / commit,
- a description and, if possible, steps to reproduce,
- the potential impact.

You'll get an acknowledgement as soon as possible, and coordinated disclosure once a fix
is available.

## Handling of secrets

- Passwords and key passphrases are stored in the **operating system keychain**
  (`keyring`), never in plaintext on disk.
- Exported configuration **excludes all secrets**.
- Logs redact credentials and never record passphrases.
- Host-key verification is enabled by default; a changed host key blocks the connection.

## Scope

In scope: the desktop application and its tunnel engine. Out of scope: vulnerabilities in
upstream dependencies (report those upstream), and misuse of the tool against systems you
are not authorized to access.
