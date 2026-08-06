# Pith Player 5.0.13

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.0.13-setup.exe` | Installer. Ships with FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.0.13-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## In this build

- **English interface.** Right-click → Other → Language. The choice is
  remembered between runs.
- **Darker theme** across the whole player, with the settings window rebuilt
  into cards and switches instead of a wall of checkboxes.
- **Subtitle look.** Colour and weight for each of the two subtitle layers,
  with a live sample shown where the subtitles really are.
- **Control bar adapts to the window.** In a narrow window the volume slider
  folds under the speaker icon and opens vertically; in a very small one only
  play, bookmark, sound and the seek bar remain.
- **Subtitle search button** next to the bookmark button, and the file
  duration moved to the right end of the seek bar.
- New logo, and an installer.

## How the cutting works

A video file is a box with two already-compressed streams inside — picture and
sound. Most tools save a piece by unpacking those streams and packing them
again: minutes of processor work and a little quality lost every time.

Pith Player copies the compressed bytes of the chosen range straight into a new
box. Nothing is decoded, nothing is compressed again, so the disk does the work
and a 20-second clip out of a 4K film lands in about a second — looking exactly
like the original, because it is the original.

The clip starts at the nearest keyframe before your mark, sometimes a fraction
of a second early; that is the price of the speed. Turn on *Re-encode instead
of remuxing* when you need the start on the exact frame.
