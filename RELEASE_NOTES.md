# Pith Player 5.1.41

A Windows video player for people who cut clips out of video. Watch, mark the
moments with `T`, press one button — every mark becomes its own video file, in
seconds, without re-encoding and without losing quality.

## Download

| File | What it is |
|---|---|
| `PithPlayer-5.1.41-setup.exe` | Installer. Offers to download FFmpeg, so cutting works out of the box. |
| `PithPlayer-5.1.41-portable.zip` | Unzip anywhere and run `pith-player.exe`. |

Windows 10 or 11, 64-bit. Nothing else to install.

The portable build needs `ffmpeg.exe` and `ffprobe.exe` next to the player (or
in `PATH`) for cutting; playback works without them.

## Bookmarks go to Notion

The marks you set while watching can now leave the player. One button in the
fragments panel sends the whole list to a Notion database: the subtitle line as
the card title, the film name, the actor, and a link to the clip.

- **One database for every film.** Rows carry a `FILM NAME` property, so a
  season of work lives in one table and filters apart by film.
- **The order you set them in.** Notion has no notion of "the order I added
  these", so the player writes a numeric `NUM` property and the view sorts by
  it. New rows continue the numbering already in the table instead of starting
  over.
- **Rows are written three at a time**, with a pause and a retry when Notion
  asks for one, and the whole export runs in the background — the player keeps
  playing while it works.
- **Cut and export in one press.** The export window can start the cutting
  queue as soon as the rows are in.

## Transcription of the English line

Each exported card can carry a phonetic transcription of its English text.
Words are looked up on wooordhunt.ru, and anything it does not know is asked of
the Cambridge dictionary — contractions like `we're` and `haven't` go there
first, because the Russian dictionary answers them with the wrong word.

Everything found is kept in a local dictionary, so the second export of the
same film asks the network for almost nothing. The dictionary warms itself up
in the background; measured against 4K playback, the frame time does not move.

## Actors

A window of the film's cast, with photographs from TMDB. Pick an actor and the
bookmarks you make are attributed to them, and the attribution travels to
Notion with the row. Photographs keep their proportions, and a click enlarges
one.

## The export window

- **Your answers are remembered** — series or film, and every toggle.
- **A journal that says where each value came from.** Colour-coded by source,
  line by line, with a button that copies the whole thing.

## The fragments panel

- **Drag its edge to any width**, up to nearly the whole window; the width is
  remembered between runs. The line you drag is one pixel wide and shows itself
  only while you drag it.
- **A close tab lives on the panel's edge.** Widened to most of the screen, the
  panel had almost no "outside" left to click.
- **Detach it into a window of its own** with the button next to the pin — for
  a second monitor. It remembers its size and position, and its close button
  docks it back into the player.
