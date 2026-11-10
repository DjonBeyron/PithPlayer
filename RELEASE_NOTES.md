# Pith Player 5.2.0

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.2.0-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.2.0-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## What is new in 5.2.0

- **Export goes only to the GitHub database (CodeEditVideo).** Notion is gone.
  The line, the film and the actor are written natively; the export warns if
  the same line for the same film is already there.
- **Bookmarks panel**: double-click the text to edit the name and the actor;
  the rename window is a window of its own, remembers its place (even on a
  second monitor), focuses the right field and can paste from the clipboard
  automatically (switchable). Removing a bookmark starts a 5-second timer with
  a countdown on the icon; click it again to cancel, it keeps running even if
  the panel is closed.
- **Cast without any key**: press A, switch to *Manual*, type the film name,
  add characters and attach a photo from the clipboard or a file. Everything is
  kept in `cast.json` and `manual-photos`, and survives restarts.
- **Text from the frame (OCR, local, open source)**: choose the subtitle zone
  in the menu once; a bookmark with no subtitle line takes the frame of that
  zone and reads the text with a progress bar in its row. If nothing is
  recognized, the default name stays. Latin script.
- **Translate subtitles with DeepL**: hover a line, click «A·Я».
- **New installer**: dark theme; replaces the installed version in the same
  folder; an unchecked-by-default option to delete old bookmark data (a backup
  is always made first, and nothing is deleted if you cancel); on uninstall it
  asks whether to remove your data (default: keep).
- Also: 8K video plays smoothly (NVDEC), instant seeks, faster file switching,
  the GitHub token survives reinstalling, developer mode (three clicks on the
  version in the Update window).

## Where your data lives

Bookmarks, settings, watch positions, the cast cache and the dictionary live in
`%APPDATA%\PithPlayer`. Installing, reinstalling and uninstalling leave them
alone.
