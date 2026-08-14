# Pith Player 5.1.45

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.1.45-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.1.45-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## The installer sets up your video files

- **A checkbox, ticked by default, opens video files with the player** — the
  same fifteen extensions the player itself knows: mkv, mp4, avi, mov, webm,
  ts, m2ts, m4v, flv, wmv, mpg, mpeg, vob, ogv, 3gp. Uninstalling removes the
  entries again.
- Windows 10 and 11 do not let a program make itself the default handler —
  that is the user's decision. The player now appears in "Open with" and in the
  list of default apps; picking it there is one click, once.
- **Reinstalling on top no longer downloads FFmpeg again** if it is already
  next to the player. Ninety megabytes saved on every update.
- Installing over a running player closes it and puts it back.

## Updates

A new **Update** window in the menu asks GitHub what has been released — one
request, under a second. It shows the version, the size of the installer and
the release notes, downloads the installer on request, and runs it when you
press the button. The player never installs anything on its own.

The check also runs quietly once at startup and says what it found with a line
in the corner rather than a window over the picture. It can be switched off in
the same window.

## Keyboard shortcuts, in one place

Until now the keys were fixed in the code, and the only way to see them was the
`--help` text.

- **Menu → Keyboard shortcuts** lists all fifteen actions with their keys.
- **Click a key and the next press replaces it.** Escape cancels, and Escape
  itself cannot be assigned — it is what closes windows.
- **A key already in use is taken off its old action**, and the window says
  which one it was.
- One button restores the original scheme.
- Modifiers still change the *step* of seeking and volume rather than the
  action: Shift for a minute, Alt for a second.
- Keys are read by physical position, so a shortcut on `]` keeps working on a
  Russian keyboard layout.

## Without a network connection

The player itself is unaffected: playback, subtitles, cutting and bookmarks
never touch the network. Cast lists, actor photographs and known transcriptions
are read from disk.

**The export now fails in a second instead of minutes.** It used to ask the
dictionaries about every unknown word before contacting Notion, and each
unanswered word costs about four seconds — a hundred words meant minutes of
waiting to be told there was no connection, when no row could have been created
anyway. Notion is now asked first.

## Where your data lives

Nothing in this release moves it, and it is worth saying plainly: bookmarks,
settings, watch positions, the cast cache and the dictionary of transcriptions
live in `%APPDATA%\PithPlayer`. Installing, reinstalling and uninstalling leave
them alone — the installer carries the player, not your data.
