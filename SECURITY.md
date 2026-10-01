# Security and responsible reports

Super Clipboard stores clipboard contents locally. Do not post real clipboard
history, passwords, backup files, logs containing personal information, or access
tokens in public issues. Reproduce bugs with synthetic data first.

For a security issue, use GitHub's private vulnerability reporting feature when
available. If unavailable, open an issue requesting a private contact without
including exploit details, credentials, or private data. The project does not
claim a guaranteed response time or offer a paid support service.

Use current Windows security updates. The application does not need administrator
privileges. A program running as the same Windows user may access clipboard data
and DPAPI-protected data; encryption does not protect an already compromised user
account. Password-encrypted library backups are separate from Windows DPAPI.

The release is currently unsigned. A SHA-256 checksum detects a mismatch with a
published file; it does not identify a trusted Windows publisher. Do not disable
Windows security features to install the application.

Only application source, tests, build configuration, licenses, and documentation
belong in the repository. Clipboard databases, personal snippets, backups,
certificates, credentials, logs, and local QA captures must never be committed.
