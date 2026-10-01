# Public website

The static player landing page is published at <https://skulitom.github.io/LookingGlass/>.

Edit the authored files in `docs/site/`. The page uses native video controls,
system fonts and HTML disclosures; no JavaScript, analytics or third-party font
service is required. It identifies the preview's limits and links directly to
the Windows package, installation help and contributor documentation.

The visual design draws on the game's Victorian menus: olive wallpaper,
parchment, oxblood accents and a brass mirror. Its repeating pattern, frame and
ornaments are newly authored CSS/SVG, not extracted game textures. Typography
uses local Palatino/Book Antiqua and Georgia, with system sans-serif for small
practical details. Decorative elements are hidden from assistive technology.
Keep the download prominent, text readable and video controls unobstructed.

The `Publish website` workflow deploys those files plus the three existing,
hash-verified gameplay MP4s and the approved story poster. It also fetches the
78-second story showcase and two 60-second chapter edits from the release,
checking each against its pinned SHA-256. It does not publish the repository,
private research or game data. The source audit allows only the four named
website files, preserving the existing media restrictions. The original artwork
in footage is excluded from MIT.

When releasing an update, review the download URL, version, size, requirements
and release-note URL in `docs/site/index.html`. Keep the preview description
consistent with the README and known issues. Verify desktop and mobile layout,
video playback, keyboard navigation, disclosures and the download destination.
When changing the theme, update the stylesheet/favicon version query in the
HTML so returning visitors receive matching styles instead of a cached theme.

For local preview, copy the four `docs/site/` files into a temporary folder.
Populate its `media/` subfolder with the same approved files as the workflow,
including the story poster and release videos. Serve that temporary folder
with a local HTTP server. Do not put preview output into the source tree.

Check widths down to 320 px, keyboard focus, the skip link, native disclosures
and all video controls. Respect reduced-motion preferences and forced colors;
avoid autoplay, decorative movement or external font dependencies.
