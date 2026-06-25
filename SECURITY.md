# Security Policy

WhatsVault handles local chat backups and media. Treat privacy bugs as security bugs.

## Supported Versions

The project is pre-alpha. Security fixes target the current `main` branch until the first public release policy is defined.

## Reporting a Vulnerability

Use GitHub private vulnerability reporting when available. If that is not available, open a public issue with only sanitized details.

Do not include:

- real message content
- contact names or phone numbers
- backup IDs
- file IDs
- full local paths
- raw SQLite rows
- private media
- real exported chat archives

Safe reports can include:

- app version
- operating system
- source type
- redacted error text
- affected file extension categories
- synthetic reproduction fixtures
- aggregate counts

## Local-Only Boundary

WhatsVault should not upload chats, backups, contacts, or media. Any future feature that transfers private data off-device must be treated as a security-sensitive architectural change and documented before implementation.
