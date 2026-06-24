# Sonido

![](assets/demo/preview.png)

## Description

A sleek, terminal-based music player written in Rust

## Table of Contents

1. [Sonido](#sonido)
2. [Description](#description)
3. [Table of Contents](#table-of-contents)
4. [Features](#features)
5. [Controls](#controls)
6. [Installation](#installation)
7. [Configuration](#configuration)
8. [Usage](#usage)
9. [Feedback](#feedback)
10. [License](#license)

## Features

* Play local audio files
* Search tracks by title
* Lightweight & fast
* Navigate with only keyboard needed
* Simple controls
* Detailed metadata
* Highly customizable

## Controls

By default, the controls are:

* `Space` -> Toggle playback
* `R` -> Toggle repeat
* `H`/`←` -> Seek backward (-5s)
* `L`/`→` -> Seek forward (+5s)
* `K`/`↑` -> Go to previous track
* `J`/`↓` -> Go to next track
* `X` -> Hide current track from playlist
* `M` -> Toggle metadata panel
* `/`/`S` -> Search
* `C` -> Reload config
* `Q` -> Quit

But you can set everything as you want. The config file is located at `~/.config/sonido/config.toml`, it will be created on first launch. If it doesn't show up, you can manually copy the [default config](assets/configs/default.toml))

## Installation

* (For Arch Linux) Install from AUR using `yay` helper
    ```Shell
    yay -S sonido
    ```
* (For other systems) Install from GitHub
    ```Shell
    git clone https://github.com/desyatkoff/sonido.git && cd sonido/ && bash ./install.sh
    ```

## Configuration

The config file will automatically created on first launch and will contain these settings:

```TOML
[config]
toggle_playback = ["space"]
toggle_repeat = ["r"]
seek_backward = ["h", "left"]
seek_forward = ["l", "right"]
seek_step = 5
previous_track = ["k", "up"]
next_track = ["j", "down"]
hide_track = ["x"]
toggle_metadata_panel = ["m"]
search = ["/", "s"]
reload_config = ["c"]
quit = ["q"]
show_app_title = true
show_playlist_title = true
show_playlist_scrollbar = true
show_metadata_title = true
show_metadata_panel = true
show_progress_title = false
show_search_title = true
app_title_format = "┤ Sonido v{VERSION} ├"
playlist_title_format = "┤ Playlist │ {PLAYLIST_PROGRESS} ├"
metadata_title_format = "┤ Metadata ├"
progress_title_format = "┤ Progress ├"
search_title_format = "┤ Search ├"
app_title_alignment = "center"
playlist_title_alignment = "left"
metadata_title_alignment = "left"
progress_title_alignment = "left"
search_title_alignment = "left"
app_title_color = "blue"
playlist_color = "blue"
metadata_color = "blue"
progress_color = "blue"
search_color = "blue"
rounded_corners = true
```

For `app_title_format` setting, the placeholder `{VERSION}` will be replaced with current app version installed

For `playlist_title_format` setting, the placeholder `{PLAYLIST_PROGRESS}` will be replaced with number of current track and total tracks count (for example: `4 / 20`)

Press `reload_config` key or restart Sonido after editing to apply changes. Everything is simple and intuitive, so it's not necessary to write a whole guide on it

Config presets you can find [here](assets/configs/) or simply make your own one

## Usage

* Get music from current directory
    ```Shell
    sonido
    ```
* Get music from `~/Music/` directory
    ```Shell
    sonido ~/Music/
    ```
* Get `track1.mp3` only, skip other files
    ```Shell
    sonido ~/Music/track1.mp3
    ```
* Get tracks from text file which contains paths (directories and audio files) separated by newlines
    ```Shell
    sonido --playlist ~/.config/sonido/playlist.txt
    ```
* Get tracks recursively from all subdirectories
    ```Shell
    sonido --recursive ~/Music/
    ```
* Get tracks sorted in alphabetical order
    ```Shell
    sonido --sort ~/Music/
    ```
* Get help
    ```Shell
    sonido --help
    ```
* Get version
    ```Shell
    sonido --version
    ```

## Feedback

Found a bug? [Open an issue](https://github.com/desyatkoff/sonido/issues/new)

## License

Copyright (C) 2026 Sergey Desyatkov

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>
