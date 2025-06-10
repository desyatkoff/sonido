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


# Features

* Play local audio files: `.mp3`, `.wav`, `.flac` and others
* Lightweight & fast
* Terminal user interface - navigate with only keyboard needed
* Playback controls – play, pause, go to previous track, go to next track
* Simple shortcuts


# Shortcuts

* `Space` -> Toggle playback
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
4. Copy compiled binary to the `/usr/bin/` directory
    ```Shell
    $ sudo cp target/release/sonido /usr/bin/
    ```
5. Test Sonido
    ```Shell
    $ sonido /path/to/playlist/directory/
    ```
