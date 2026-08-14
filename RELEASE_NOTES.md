# Pith Player 5.1.50

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.1.50-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.1.50-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## Fixed: the arrow keys work again

**If you installed 5.1.48, replace it with this build.** Seeking with the arrow
keys and changing the volume did nothing in that release.

The cause was a mismatch of names. Shortcuts became reassignable in 5.1.45, so
the scheme now stores each key by name — and the name written for the arrows
(`ArrowRight`) was not the name the interface toolkit uses for the same key
(`Right`). The two never met, so the binding was silently never found. Space,
letters and Backspace kept working because their names happen to be identical,
which is exactly why the breakage went unnoticed.

Name translation now lives in one place and works both ways — when a key is
looked up and when it is written down. Both spellings are accepted, so a scheme
saved by 5.1.48 keeps working and nothing has to be reassigned.

A test now walks every one of the fifteen actions and demands that its key find
its action, going through the toolkit the way a real keypress does. Two keys out
of fifteen, checked by hand, proved nothing.

## Everything else, from 5.1.48 and 5.1.45

- **Actors in Russian where Russian exists**: missing names are asked of
  Wikidata by the film database's person id — one request per cast, about a
  second. Right-click an actor to fix a name by hand; there is deliberately no
  automatic transliteration.
- **A dictionary of transcriptions ships with the player**, so a fresh install
  starts with words instead of an empty file. Your own words are never
  overwritten.
- **The installer registers video file types** — fifteen extensions, a checkbox
  ticked by default, removed again on uninstall. Reinstalling no longer
  re-downloads FFmpeg.
- **Updates** can be checked and installed from the menu, and are checked
  quietly once at startup.
- **Keyboard shortcuts** can be seen and reassigned in one window.
- **Export without a network connection** fails in a second instead of minutes.

## Where your data lives

Bookmarks, settings, watch positions, the cast cache and the dictionary live in
`%APPDATA%\PithPlayer`. Installing, reinstalling and uninstalling leave them
alone — the installer carries the player, not your data.
