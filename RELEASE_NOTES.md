# Pith Player 5.1.24

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.1.24-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.1.24-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## Cut clips no longer start silent

Cutting copies the picture and, by default, re-encodes the sound to AAC so that
Adobe editors can read it. FFmpeg treats those two differently: the copied
picture starts at the keyframe before your mark, while the re-encoded sound was
trimmed exactly at the mark. Measured on a 4K film, the picture began at 0.08 s
and the sound only at 2.06 s — the first two seconds had no audio, and the
opening looked jerky because the player had nothing to sync the picture to.
Now both start together, within 0.02 s of each other.

Two more things came out of the same investigation:

- **Clips are closer to the length you asked for.** The player used to look up
  the nearest keyframe itself and start exactly there — which made FFmpeg step
  back one keyframe further. An 18-second clip came out 20.2 s long; it is now
  19.0 s, and one lookup per batch is gone with it.
- **No more phantom chapter track.** FFmpeg copied the source film's chapters
  into every clip as a text track spanning the whole original — a ten-minute
  track inside an eighteen-second file.

## The installer now fetches FFmpeg

Cutting clips is done by FFmpeg, and the installer used to carry whatever copy
the build machine had — which turned out to be a redirecting shim, useless on
anyone else's computer. The installer now offers to download FFmpeg itself:
the build recommended on ffmpeg.org, verified against the checksum published
next to it. Decline it and the player still installs and plays; only cutting
stays unavailable.

## Opening a file is twice as fast

Measured from process start to the first frame on screen: **1.4 s → 0.7 s** for
a 20 GB 4K film, and the same for a small clip — file size barely mattered,
because the time was going elsewhere.

- The player wrote its port into `instance.port` and cleaned it up in a place
  the code never reached. Every launch but the very first waited out the full
  connection timeout to a port that was long gone: **610 ms of pure nothing.**
- Subtitle lines were polled from mpv on every frame, including while the file
  was still opening. The engine is busy then, and the answer takes its time —
  **283 ms of frozen interface** for lines that did not exist yet.
- The seek preview started itself in the first frame: a second mpv instance and
  a thumbnail mosaic competing for the processor exactly while the main engine
  was opening the file. Now they start when you first hover the seek bar.

## The window no longer jumps

It used to appear at the previous session's size and half a second later snap
to the shape of the video. The shape is now read from the container before the
window is created, so it opens correct from the first moment — a vertical video
opens in a vertical window straight away.

## Language fixes

- Choosing English in the installer now actually gives you an English player.
- On a clean machine the player follows the language of Windows.
- The default fragment list reads as **Main**, not «Основной».

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
