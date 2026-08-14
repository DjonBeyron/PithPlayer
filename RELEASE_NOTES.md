# Pith Player 5.1.48

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.1.48-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.1.48-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## Actors, in Russian where Russian exists

Cast names arrive translated for the people someone has written about — 17 of
34 on one film, 10 of 47 on a recent one. The rest used to stay in Latin
script with nowhere to get a name from.

- **Missing names are now asked of Wikidata**, matched by the film database's
  own person id rather than by spelling, so namesakes cannot be confused. One
  request for the whole cast, about a second. Measured: three names recovered
  out of seventeen on one film, none on a very recent one — nobody has written
  about those actors in Russian anywhere.
- **Right-click an actor to fix the name by hand.** What you type is kept next
  to the film and travels to Notion. There is deliberately no automatic
  transliteration: a machine would produce confident, plausible, wrong
  spellings, and you could not tell them from the right ones.

Existing cast lists keep the names they were saved with; refresh a cast in the
actors window to pick up the new ones.

## A dictionary that ships with the player

Transcriptions are collected while you watch and kept on your machine. Half of
any film's lines, though, are words every film uses — so the words collected so
far now travel inside the player itself. A fresh install starts with a
dictionary instead of an empty file, and your own words are never overwritten:
only what is missing gets added, once.

## Also in this release

Everything from 5.1.45, in case you skipped it: the installer registers video
file types, updates can be checked and installed from the menu, keyboard
shortcuts can be seen and reassigned in one window, and export without a
network connection fails in a second instead of minutes.

## Where your data lives

Bookmarks, settings, watch positions, the cast cache and the dictionary live in
`%APPDATA%\PithPlayer`. Installing, reinstalling and uninstalling leave them
alone — the installer carries the player, not your data.
