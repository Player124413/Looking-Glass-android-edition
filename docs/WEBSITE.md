# Public website

The static player landing page is published at <https://skulitom.github.io/LookingGlass/>.

Edit the authored files in `docs/site/`. The page uses native video controls,
system fonts and HTML disclosures; no JavaScript, analytics or third-party font
service is required. It identifies the preview's limits and links directly to
the Windows package, installation help and contributor documentation.

The `Publish website` workflow deploys those files plus the three existing,
hash-verified gameplay MP4s. It does not publish the repository, private research,
game data or new captures. The source audit allows only the four named website
files, preserving the existing media restrictions. The original artwork in
footage is excluded from MIT.

When releasing an update, review the download URL, version, size, requirements
and release-note URL in `docs/site/index.html`. Keep the preview description
consistent with the README and known issues. Verify desktop and mobile layout,
video playback, keyboard navigation, disclosures and the download destination.

For local preview, copy the four `docs/site/` files into a temporary folder and
the three approved MP4s into its `media/` subfolder. Serve that temporary folder
with a local HTTP server. Do not put preview output into the source tree.
