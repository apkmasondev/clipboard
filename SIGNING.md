# Code signing policy

The current Windows release is unsigned. No code-signing certificate has been
issued to this project, and no signing service is configured. No claim of
SignPath sponsorship, Microsoft certification, or SmartScreen reputation is made.

The owner selected a license permitting free professional use while prohibiting
sale of the application and its modifications. This is not an OSI-approved
open-source license. SignPath Foundation currently requires an OSI-approved
license and project approval, so its free OSS signing program is not a compatible
route under the present license.

References:
- https://signpath.org/terms.html
- https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation

Releases should be built from a tagged source revision after Windows tests pass.
Publish the installer and its SHA-256 checksum together in GitHub Releases.
Never commit signing keys or tokens. A future trusted signing integration must
verify both the application and installer signatures and use timestamping.
