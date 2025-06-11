# Sonido

```
 ____              _     _       
/ ___|  ___  _ __ (_) __| | ___  
\___ \ / _ \| '_ \| |/ _` |/ _ \ 
 ___) | (_) | | | | | (_| | (_) |
|____/ \___/|_| |_|_|\__,_|\___/
```


# Description

A sleek, terminal-based music player written in Rust


# Table of Contents

1. [Sonido](#sonido)
2. [Description](#description)
3. [Table of Contents](#table-of-contents)
4. [Features](#features)
5. [Controls](#controls)
6. [Installation](#installation)
7. [Usage](#usage)
9. [Feedback](#feedback)
10. [License](#license)


# Features

* Play local audio files: .mp3, .wav, .flac and others
* Lightweight & fast
* Terminal user interface - navigate with only keyboard needed
* Playback controls. Play/Pause, go to previous/next track, seek backward/forward
* Simple controls
* Detailed metadata


# Controls

* `Space` -> Toggle playback
* `←` -> Seek backward
* `→` -> Seek forward
* `↑` -> Go to previous track
* `↓` -> Go to next track
* `Q` -> Quit


# Installation

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
4. Copy compiled binary to the /usr/bin/ directory
    ```Shell
    $ sudo cp target/release/sonido /usr/bin/
    ```


# Usage

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
* Get music from ~/Music/
    ```Shell
    $ sonido ~/Music/
    ```
* Get music recursively from ~/Music/
    + Short
        ```Shell
        $ sonido -r ~/Music/
        ```
    + Full
        ```Shell
        $ sonido --recursive ~/Music/
        ```


# Feedback  

Found a bug? [Open an issue](https://github.com/desyatkoff/sonido/issues/new)


# License

Copyright (C) 2025 Desyatkov Sergey

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version. This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with this program. If not, see <https://www.gnu.org/licenses/>
