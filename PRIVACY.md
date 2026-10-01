# Privacy

Super Clipboard has no account system, telemetry, advertising, AI service, or
remote clipboard storage. The running application does not make network requests.
The installer may obtain the Microsoft WebView2 runtime if it is missing; build
tools and GitHub separately use their own network services.

Clipboard history is saved automatically unless paused or excluded. You can
exclude executable names, unknown sources, or data types. Built-in sensitive
format handling and optional token detection reduce accidental storage but do not
recognize every password or banking browser tab. Exclusions apply to future
copies. You can remove earlier history in the application.

Content and descriptive metadata are encrypted with Windows DPAPI for the current
user. Technical IDs, type, time, size, and pin/library flags remain in SQLite.
The full-text index is in memory. Images are separate encrypted files. The Windows
pagefile, crash dumps, or the system clipboard may independently contain data.

Snippet revision history keeps up to ten previous versions per snippet, subject
to a 64 MB global encrypted revision budget. Deleting a snippet deletes its
revisions. A migration from database version 1 creates a local encrypted database
snapshot named `migration-v1.sqlite`; this recovery file is not automatically
removed when history is cleared. Removing all application data during uninstall
also removes this file.

JSON export contains plaintext snippets. Password-protected `.scbackup` files
contain only the current snippet library and favorites, not clipboard history,
settings, or revisions. Backup passwords are not stored. The application cannot
recover a forgotten backup password. Restoring a library adds missing snippets,
preserves existing data, and disables imported shortcuts.

No local data is uploaded to the GitHub repository or included in installers.
