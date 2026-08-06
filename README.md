<img src="assets/logo.png" width="88" align="right" alt="Pith Player logo">

# Pith Player

A Windows video player for people who cut clips out of video.
Watch, mark the moments you want, get them as separate files — in seconds,
without re-encoding and without losing quality.

## Download

**[Download the latest release](https://github.com/DjonBeyron/PithPlayer/releases/latest)**
— Windows 10 or 11, 64-bit.

| File | What it is |
|---|---|
| `PithPlayer-x.y.z-setup.exe` | Installer. Run it, click through, done. |
| `PithPlayer-x.y.z-portable.zip` | Unzip anywhere and run `pith-player.exe`. No installation. |

Cutting clips needs **FFmpeg** — `ffmpeg.exe` and `ffprobe.exe` either next to
the player or anywhere in `PATH`. The release usually ships with them. Without
FFmpeg the player still plays everything; only cutting is unavailable.

## What it does

Open a video. Press `T` at any moment worth keeping — that drops a bookmark.
Keep watching, drop as many as you like. When you are done, press one button
and every bookmark becomes its own video file on disk.

Each bookmark turns into a clip of a fixed length that starts a little before
the mark — you set both numbers once (say, 18 seconds long, starting 5 seconds
early), so you can mark on reaction instead of aiming at the exact frame.
Bookmarks live in named lists, each list with its own length, lead-in and
output folder, and each list has its own colour on the timeline.

## How the cutting works, and why it is instant

A video file is a box. Inside it there are two streams — picture and sound —
already compressed, once, by whoever made the file.

Most tools save a piece by unpacking those streams into raw frames and packing
them again. That is re-encoding: minutes of work for your processor, heat, fan
noise, and a little quality lost every single time.

Pith Player does not touch the streams at all. It copies the compressed bytes
of the chosen range straight into a new box, byte for byte. Nothing is decoded,
nothing is compressed again — it is a copy operation, so the disk does the work
and the processor stays idle. A 20-second clip out of a 4K film lands in about
a second, and it looks exactly like the original, because it **is** the
original.

There is one honest catch. Compressed video can only start playing from a
keyframe — a full picture that later frames are built from. Those sit a couple
of seconds apart, so a clip starts at the nearest keyframe **before** your
mark: sometimes a fraction of a second earlier than asked. That is the price of
the speed. If you need the start on the exact frame, turn on *Re-encode instead
of remuxing* in the settings, and the clip is rebuilt precisely — slower, the
usual way.

Two more things keep it quick: the file is measured once when you open it, not
once per bookmark, and several clips are written at the same time.

## Also in the box

- **Subtitle search.** `Ctrl+F`, type a phrase, jump to the line — or bookmark
  it straight from the results, which is usually why you were looking.
- **Two subtitle layers** at once, with your own colour, weight, size and
  position for each. Drag them with the mouse, resize with the wheel.
- Speed without chipmunk voices, loop, frame preview while seeking.
- Crop black bars in fullscreen, window shaped to fit the video.
- Resume where you stopped, recent files, one window for every file you open.
- Interface in **English and Russian** — right-click → Other → Language.

## Hotkeys

| Key | Action |
|---|---|
| `Space` | Pause / play |
| `←` `→` | Seek (hold `Shift` for small steps, `Ctrl` for big ones) |
| `↑` `↓` | Volume |
| `F` | Fullscreen |
| `T` | Bookmark here |
| `C` | Copy the current subtitle line |
| `Ctrl+F` | Search subtitles |

Right-click the video for everything else.

## Building from source

See [BUILDING.md](BUILDING.md) — Rust, libmpv, one `cargo build`.

Development notes live in [PLAN.md](PLAN.md) and [CLAUDE.md](CLAUDE.md),
both in Russian.
