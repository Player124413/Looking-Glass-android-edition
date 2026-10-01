# Code signing policy

## Current status

Looking Glass is preparing an application for free code signing from
[SignPath Foundation](https://signpath.org/). **The current Windows preview is
unsigned.** No SignPath approval, certificate or signed release is claimed.
Its checksums verify file integrity; they do not establish a verified publisher.

If approved and integrated, the signing credit will be: Free code signing
provided by [SignPath.io](https://signpath.io/), certificate by
[SignPath Foundation](https://signpath.org/).

## Responsibility and release approval

[Artem Skulimovskiy (skulitom)](https://github.com/skulitom) is the project's
maintainer, author, reviewer and designated release-signing approver. Changes
from other contributors require maintainer review. Every future signing request
requires explicit maintainer approval. Signing must not be enabled until
multifactor authentication and the provider's required access controls are set
up for the source repository and signing account.

The current GitHub Actions workflow checks source, setup and Rust tests. The
first preview was built locally; it is not presented as a SignPath-verifiable
automated release build. Future signed releases must be built from the public
source through a verifiable build pipeline, with product/version metadata,
signature verification and new checksums recorded for the distributed files.

Signing will cover Looking Glass's own binaries and installer. Original Alice
game data, original executables, gameplay publicity media and private research
are excluded from signing requests. Third-party helpers retain their own
identities and notices and will not be signed as Looking Glass-authored code.

## Privacy and network behavior

The current engine and setup do not send analytics, advertising identifiers,
crash reports, saves or personal files to the project maintainer. Saves, settings,
setup logs and remembered local paths stay on the player's computer, normally
under the installation's `private/` folder. Logs can contain local paths; review
them before voluntarily including information in a public bug report.

Setup contacts Internet Archive only when the player selects the optional game
data download and starts setup. That service receives the network request,
including the connection's IP address and the setup application's user-agent.
Choosing existing local game files avoids this download. Visiting GitHub,
Internet Archive or SignPath in a browser is subject to those services' own
privacy policies. Looking Glass does not automatically upload anything to them.

To remove this portable preview, close it and remove its installation folder
and any optional desktop shortcut. Back up saves first: deleting the folder also
deletes saves and imported data stored there. A separately selected original
game-data folder is unaffected. See [installation and removal](INSTALL.md).
