# Pith Player 5.1.58

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.1.58-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.1.58-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## Updates that actually find you

- **A window opens at startup when a new version is out**, with the release
  notes and a **Remind me tomorrow** button. Close it without choosing and it
  asks again next launch.
- **The installer tells you it is replacing an older version** and saves copies
  of your data files (`%APPDATA%\PithPlayer\backup\<old version>`) before
  touching anything. Your bookmarks, settings and dictionary are never part of
  the installation.

## Export to CodeEditVideo

Menu → Integrations → **Where to export**: Notion or CodeEditVideo. With
CodeEditVideo the line, the film name, the actor and the transcription go
straight into `db/records.json` of the projects repository on GitHub. It needs
a GitHub token with write access to that repository; **Check access** only
reads and writes nothing.

## Playback

- **No more freeze on the last frame** when you open the next video after one
  finished: the file is now opened off the interface thread.
- **8K video** uses lighter scaling (bilinear, no dithering or debanding) so
  the graphics card keeps up. Smaller video looks exactly as before.

- **The metrics panel shows the real decoder and codec**, not just the mode
  that was requested, so a silent fall back from hardware decoding is visible.
- **The installer puts the new version over the old folder**, even if the old
  copy was installed in a different mode (for all users / only me).

- **A token that passed "Check access" is saved at once**, so closing the
  window no longer loses it.

- **NVDEC is now the first decoder choice** (zero-copy through CUDA). The
  logs showed `d3d11va` never started with OpenGL rendering, so every frame
  was copied through memory — too slow for 8K.
- **Seeking in 8K is instant**: a click on the timeline first shows the nearest
  keyframe, then refines to the exact spot in the background.
- **Developer mode**: click the version number three times in the Update
  window to choose a folder for the logs (`pith-player.log`, `mpv.log`).

## Where your data lives

Bookmarks, settings, watch positions, the cast cache and the dictionary live in
`%APPDATA%\PithPlayer`. Installing, reinstalling and uninstalling leave them
alone.
