# Sonido

```
 ____              _     _       
/ ___|  ___  _ __ (_) __| | ___  
\___ \ / _ \| '_ \| |/ _` |/ _ \ 
 ___) | (_) | | | | | (_| | (_) |
|____/ \___/|_| |_|_|\__,_|\___/
```


## Description

A sleek, terminal-based music player written in Rust


## Table of Contents

1. [Sonido](#sonido)
2. [Description](#description)
3. [Table of Contents](#table-of-contents)
4. [Features](#features)
5. [Controls](#controls)
6. [Installation](#installation)
7. [Usage](#usage)
8. [Configuration](#configuration)
9. [Feedback](#feedback)
10. [License](#license)


## Features

* Play local audio files
* Lightweight & fast
* Navigate with only keyboard needed
* Simple controls
* Detailed metadata
* Highly customizable


## Controls

By default, the controls are:

* `Space` -> Toggle playback
* `←` -> Seek backward (-5s)
* `→` -> Seek forward (+5s)
* `↑` -> Go to previous track
* `↓` -> Go to next track
* `Q` -> Quit

But you can set everything as you want. The config file is located at `~/.config/sonido/config.toml`, it will be created on first launch. If it doesn't show up, you can manually copy the [default config](assets/configs/default.toml))


## Installation

1. Clone the repository
    ```Shell
    $ git clone https://github.com/desyatkoff/sonido.git
    ```
2. Go to the repository directory
    ```Shell
    $ cd sonido/
    ```
3. Compile the Rust project
    ```Shell
    $ cargo build --release
    ```
4. Copy compiled binary to the `/usr/bin/` directory
    ```Shell
    $ sudo cp target/release/sonido /usr/bin/
    ```


## Usage

* Get music from current working directory
    ```Shell
    $ sonido
    ```
* Get music recursively from current working directory (from all subdirectories)
    + Short
        ```Shell
        $ sonido -r
        ```
    + Full
        ```Shell
        $ sonido --recursive
        ```
* Get music from `~/Music/`
    ```Shell
    $ sonido ~/Music/
    ```
* Get music recursively from `~/Music/`
    + Short
        ```Shell
        $ sonido -r ~/Music/
        ```
    + Full
        ```Shell
        $ sonido --recursive ~/Music/
        ```


## Configuration

The config file will automatically created on first launch and will contain these settings:

```TOML
[config]
toggle_playback = "space"
seek_backward = "left"
seek_forward = "right"
seek_step = 5
previous_track = "up"
next_track = "down"
quit = "q"
show_app_title = true
show_playlist_title = true
show_metadata_title = true
show_metadata_panel = true
show_progress_title = false
app_title_alignment = "center"
playlist_title_alignment = "left"
metadata_title_alignment = "left"
progress_title_alignment = "left"
rounded_corners = true
accent_color = "blue"
```

Restart Sonido after editing to apply changes. Everything is simple and intuitive, so it's not necessary to write a whole guide on it

Config presets you can find [here](assets/configs/) or simply make your own one


## Feedback  

Found a bug? [Open an issue](https://github.com/desyatkoff/sonido/issues/new)


## License

Copyright (C) 2025 Desyatkov Sergey

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>
