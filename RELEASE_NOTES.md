# Pith Player 5.1.39

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.1.39-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.1.39-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## The player no longer freezes while a file opens

Opening a 4K HDR film used to stall the window for a moment — long enough to
see it, not long enough to explain. Every stall had the same cause: the player
asked mpv for a property and waited for the answer on the interface thread,
while mpv was busy parsing the container and starting the decoder.

Measured on a 4K HDR film, frame by frame:

| What was asked | When | Frozen for |
|---|---|---|
| Playback state, six properties per frame | before the file loaded | 488 ms |
| Playback state | right after it loaded | 200 ms |
| Decoder mode, for the log | on load | 227 ms |
| Audio summary, for the log | on load | 227 ms |
| Track list | on load | 233 ms |
| Subtitle lines, twice per frame | after load | 197 ms |
| Frame size | switching files in a running player | 198 ms |

Position, volume, subtitle lines and frame size now arrive from mpv as events —
nothing is polled any more. The track list is read in one property instead of
seven per track. Everything else waits for the moment mpv says it has started
playing, when the same questions cost single milliseconds.

No frame of the interface now runs over budget while a file opens, in either
direction: launching with a file, or dropping a new file into a running player.

## Seeking with the arrow keys

- **Hold an arrow and it keeps seeking** until you let go. The step follows the
  modifier, so you get three speeds.
- **Shift + arrows now step by a minute, Alt + arrows by a second**, plain
  arrows by five seconds.
- **A burst of presses no longer queues up.** Twelve quick presses used to take
  1.47 s to settle and moved the picture twice; now it is 0.67 s and the picture
  follows every press. A single press went from 307 ms to about 170 ms.

The old measurement of seek time was lying: it closed on the next interface
frame instead of waiting for mpv, and reported single milliseconds. It now
closes when mpv reports it is ready to play from the new spot.

## The window opens the way you left it

- **Maximized stays maximized.** Close the player full screen and it opens full
  screen — instantly, with no animation and no rebuild of the interface. It is
  created at exactly the rectangle Windows itself would use, down to the pixel.
- **The title-bar button gives back your window.** Restoring returns the size
  *and* position you had before maximizing, not something near-fullscreen.
- **Nothing shifts after the window appears.** Frame-by-frame capture of the
  first two seconds shows no movement at all — only the video and the clock.

## Fragments panel

- **Copy the name of a bookmark** with the new button next to the scissors. The
  name is usually a line of dialogue, and it is truncated in the list, so it
  could not be copied by hand.
- **Pin the panel.** The pin in the corner keeps the panel open when you click
  the video; grey means it closes as before.

## Subtitle lines make better bookmark names

- **Dashes no longer come along.** Dialogue lines start with a dash — hyphen,
  en dash, em dash, whichever the subtitler used — and it ended up in the
  bookmark name, in the file name of the clip, and in the clipboard.
  `- Hi. Hi. / - (sloshing)` now becomes `Hi. Hi. (sloshing)`. A dash inside a
  sentence is punctuation and is left alone.
- **A bookmark set in a pause takes the line that just ended**, if it faded less
  than eight seconds ago. After a longer silence the bookmark stays unnamed
  rather than borrowing an unrelated line.
- **The notice no longer breaks into a column.** A long name used to arrive one
  word per line.
