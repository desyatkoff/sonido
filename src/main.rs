/*
Copyright (C) 2025 Desyatkov Sergey
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version
*/

use std::{
    env,
    path::{
        Path,
        PathBuf,
    },
    time::{
        Duration,
        Instant,
    },
};
use anyhow::Result;
use crossterm::{
    event::{
        self,
        Event,
        KeyCode,
        KeyEventKind,
    },
    execute,
    terminal::{
        disable_raw_mode,
        enable_raw_mode,
        EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use directories::ProjectDirs;
use lofty::{
    read_from_path,
    file::AudioFile,
    file::TaggedFileExt,
    tag::Accessor,
};
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        Block,
        Borders,
        Gauge,
        List,
        ListItem,
        ListState,
        Paragraph,
        Wrap,
    },
    Frame,
};
use rodio::{
    Decoder,
    OutputStream,
    Sink,
    Source,
};
use serde::{
    Deserialize,
    Serialize,
};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Track {
    path: PathBuf,
    duration: Duration,
    metadata: Metadata,
}

#[derive(Default)]
struct Metadata {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    year: Option<String>,
    genre: Option<String>,
    track_number: Option<u32>,
    bitrate: Option<u32>,
    sample_rate: Option<u32>,
    channels: Option<u8>,
}

impl Metadata {
    fn from_path(path: &Path) -> Self {
        let mut metadata = Self::default();

        let file_name = path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();

        if let Ok(tagged_file) = read_from_path(path) {
            let tag = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag());

            if let Some(tag) = tag {
                metadata.title = tag.title().map(|s| s.to_string());
                metadata.artist = tag.artist().map(|s| s.to_string());
                metadata.album = tag.album().map(|s| s.to_string());
                metadata.year = tag.year().map(|y| y.to_string());
                metadata.genre = tag.genre().map(|s| s.to_string());
                metadata.track_number = tag.track();
            }

            let properties = tagged_file.properties();

            metadata.bitrate = properties.audio_bitrate();
            metadata.sample_rate = properties.sample_rate();
            metadata.channels = properties.channels();
        }

        if metadata.title.is_none() {
            if let Some((artist, title)) = file_name.split_once(" - ") {
                metadata.title = Some(title.to_string());
                metadata.artist = Some(artist.to_string());
            } else {
                metadata.title = Some(file_name);
            }
        }

        return metadata;
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    config: ConfigSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ConfigSettings {
    toggle_playback: String,
    seek_backward: String,
    seek_forward: String,
    seek_step: u64,
    previous_track: String,
    next_track: String,
    quit: String,
    show_metadata_panel: bool,
    rounded_corners: bool,
    accent_color: String,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        ConfigSettings {
            toggle_playback: "space".into(),
            seek_backward: "left".into(),
            seek_forward: "right".into(),
            seek_step: 5,
            previous_track: "up".into(),
            next_track: "down".into(),
            quit: "q".into(),
            show_metadata_panel: true,
            rounded_corners: true,
            accent_color: "blue".into(),
        }
    }
}

struct App {
    tracks: Vec<Track>,
    config: ConfigSettings,
    current_track: usize,
    list_state: ListState,
    playback_state: PlaybackState,
    position: Duration,
    playback_start: Option<Instant>,
    sink: Option<Sink>,
    _stream: Option<OutputStream>,
}

enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let (help, recursive, version, music_directory) = parse_args(&args);
    let tracks = scan_music_files(&music_directory, recursive)?;

    if help {
        println!(
            r#"
USAGE:
    sonido [OPTIONS] [PATH]

OPTIONS:
    -h, --help       Print this help message
    -r, --recursive  Get music files from all subdirectories
    -v, --version    Print version
            "#
        );

        return Ok(());
    } else if version {
        println!(
            r#"
 ____              _     _       
/ ___|  ___  _ __ (_) __| | ___  
\___ \ / _ \| '_ \| |/ _` |/ _ \ 
 ___) | (_) | | | | | (_| | (_) |
|____/ \___/|_| |_|_|\__,_|\___/

Sonido v{}
A sleek, terminal-based music player written in Rust

Copyright (C) 2025 Desyatkov Sergey

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version
            "#,
            VERSION
        );

        return Ok(());
    }

    if tracks.is_empty() {
        anyhow::bail!("No music files found in {}", music_directory.display());
    }

    enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let config = load_config();

    let mut app = App {
        tracks,
        config,
        current_track: 0,
        list_state: ListState::default().with_selected(Some(0)),
        playback_state: PlaybackState::Stopped,
        position: Duration::ZERO,
        playback_start: None,
        sink: None,
        _stream: None,
    };

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    return result;
}

fn parse_args(args: &[String]) -> (bool, bool, bool, PathBuf) {
    let mut help = false;
    let mut recursive = false;
    let mut version = false;
    let mut music_directory = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                help = true;
            },
            "-r" | "--recursive" => {
                recursive = true;
            },
            "-v" | "--version" => {
                version = true;
            }
            _ if arg.starts_with('-') => {},
            _ => {
                if music_directory.is_none() {
                    music_directory = Some(PathBuf::from(arg));
                }
            }
        }
    }

    let music_directory = music_directory.unwrap_or_else(|| env::current_dir().unwrap());

    return (help, recursive, version, music_directory);
}

fn parse_key(key_str: &str) -> KeyCode {
    match key_str.to_lowercase().as_str() {
        "space" => {
            return KeyCode::Char(' ');
        },
        "left" => {
            return KeyCode::Left;
        },
        "right" => {
            return KeyCode::Right;
        },
        "up" => {
            return KeyCode::Up;
        },
        "down" => {
            return KeyCode::Down;
        },
        "escape" | "esc" => {
            return KeyCode::Esc;
        },
        "tab" => {
            return KeyCode::Tab;
        },
        "backspace" => {
            return KeyCode::Backspace;
        },
        "enter" => {
            return KeyCode::Enter;
        },
        "insert" | "ins" => {
            return KeyCode::Insert;
        },
        "delete" | "del" => {
            return KeyCode::Delete;
        },
        "home" => {
            return KeyCode::Home;
        },
        "end" => {
            return KeyCode::End;
        },
        "pageup" | "pgup" => {
            return KeyCode::PageUp;
        },
        "pagedown" | "pgdown" => {
            return KeyCode::PageDown;
        },
        key if key.len() == 1 => {
            return KeyCode::Char(
                key
                    .chars()
                    .next()
                    .unwrap()
            );
        },
        _ => {
            return KeyCode::Null;
        },
    }
}

fn parse_color(color_str: &str) -> Color {
    match color_str.to_lowercase().as_str() {
        "black" => {
            return Color::Black;
        },
        "red" => {
            return Color::Red;
        },
        "green" => {
            return Color::Green;
        },
        "yellow" => {
            return Color::Yellow;
        },
        "blue" => {
            return Color::Blue;
        },
        "magenta" => {
            return Color::Magenta;
        },
        "cyan" => {
            return Color::Cyan;
        },
        "gray" | "grey" => {
            return Color::Gray;
        },
        "darkgray" | "darkgrey" => {
            return Color::DarkGray;
        },
        "lightred" => {
            return Color::LightRed;
        },
        "lightgreen" => {
            return Color::LightGreen;
        },
        "lightyellow" => {
            return Color::LightYellow;
        },
        "lightblue" => {
            return Color::LightBlue;
        },
        "lightmagenta" => {
            return Color::LightMagenta;
        },
        "lightcyan" => {
            return Color::LightCyan;
        },
        "white" => {
            return Color::White;
        },
        _ => {
            return Color::Blue;
        },
    }
}

fn load_config() -> ConfigSettings {
    if let Some(project_dirs) = ProjectDirs::from("", "", "sonido") {
        let config_directory = project_dirs.config_dir();
        let config_path = config_directory.join("config.toml");

        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            match toml::from_str::<Config>(&contents) {
                Ok(config) => {
                    return config.config;
                },
                Err(e) => {
                    return ConfigSettings::default();
                }
            }
        } else {
            let default_config = ConfigSettings::default();

            std::fs::create_dir_all(config_directory);

            match toml::to_string(&Config { config: default_config.clone() }) {
                Ok(toml_str) => {
                    std::fs::write(&config_path, toml_str);
                },
                Err(_) => {},
            }

            return default_config;
        }
    } else {
        return ConfigSettings::default();
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        _ if key.code == parse_key(&app.config.quit) => {
                            return Ok(());
                        },
                        _ if key.code == parse_key(&app.config.toggle_playback) => {
                            toggle_playback(app);
                        },
                        _ if key.code == parse_key(&app.config.seek_backward) => {
                            seek(app, -(app.config.seek_step as i64));
                        },
                        _ if key.code == parse_key(&app.config.seek_forward) => {
                            seek(app, app.config.seek_step as i64);
                        },
                        _ if key.code == parse_key(&app.config.previous_track) => {
                            next_track(app, -1);
                        },
                        _ if key.code == parse_key(&app.config.next_track) => {
                            next_track(app, 1);
                        },
                        _ => {},
                    }
                }
            }
        }

        if let (PlaybackState::Playing, Some(start_time)) = (&app.playback_state, app.playback_start) {
            app.position = start_time.elapsed();

            if app.position >= app.tracks[app.current_track].duration {
                next_track(app, 1);
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let show_metadata_panel = app.config.show_metadata_panel;
    let rounded_corners = app.config.rounded_corners;
    let accent_color = parse_color(&app.config.accent_color);

    let border_set = if app.config.rounded_corners {
        border::ROUNDED
    } else {
        border::PLAIN
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    let center_layout = if show_metadata_panel {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(layout[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(layout[1])
    };

    let track = &app.tracks[app.current_track];

    let title = Block::default()
        .borders(Borders::ALL)
        .border_set(border_set)
        .border_style(Style::default().fg(accent_color))
        .title(format!(" Sonido v{} ", VERSION))
        .title_alignment(Alignment::Center);

    f.render_widget(title, layout[0]);

    let items: Vec<ListItem> = app
        .tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let display_name = track
                .metadata
                .title
                .as_ref()
                .cloned()
                .unwrap_or_else(|| {
                    track
                        .path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string()
                });
            
            let style = if i == app.current_track {
                Style::default().fg(accent_color)
            } else {
                Style::default()
            };
            
            return ListItem::new(display_name).style(style);
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border_set)
                .border_style(Style::default().fg(accent_color))
                .title(" Playlist "),
        )
        .highlight_style(Style::default().bold());

    f.render_stateful_widget(list, center_layout[0], &mut app.list_state.clone());

    let metadata = &track.metadata;
    
    let mut lines = vec![
        Line::from(vec![
            Span::styled("Title: ", Style::default().fg(accent_color)),
            Span::raw(
                metadata
                    .title
                    .as_deref()
                    .unwrap_or("Unknown")
            ),
        ]),
        Line::from(vec![
            Span::styled("Artist: ", Style::default().fg(accent_color)),
            Span::raw(
                metadata
                    .artist
                    .as_deref()
                    .unwrap_or("Unknown")
            ),
        ]),
        Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(accent_color)),
            Span::raw(format_duration(track.duration)),
        ]),
    ];
    
    if let Some(album) = &metadata.album {
        lines.push(Line::from(vec![
            Span::styled("Album: ", Style::default().fg(accent_color)),
            Span::raw(album),
        ]));
    }
    
    if let Some(year) = &metadata.year {
        lines.push(Line::from(vec![
            Span::styled("Year: ", Style::default().fg(accent_color)),
            Span::raw(year),
        ]));
    }
    
    if let Some(genre) = &metadata.genre {
        lines.push(Line::from(vec![
            Span::styled("Genre: ", Style::default().fg(accent_color)),
            Span::raw(genre),
        ]));
    }
    
    if let Some(track_num) = metadata.track_number {
        lines.push(Line::from(vec![
            Span::styled("Track: ", Style::default().fg(accent_color)),
            Span::raw(track_num.to_string()),
        ]));
    }

    if let Some(bitrate) = metadata.bitrate {
        lines.push(Line::from(vec![
            Span::styled("Bitrate: ", Style::default().fg(accent_color)),
            Span::raw(format!("{} kbps", bitrate)),
        ]));
    }

    if let Some(sample_rate) = metadata.sample_rate {
        lines.push(Line::from(vec![
            Span::styled("Sample Rate: ", Style::default().fg(accent_color)),
            Span::raw(format!("{} Hz", sample_rate)),
        ]));
    }

    if let Some(channels) = metadata.channels {
        let channel_str = match channels {
            1 => "Mono".to_string(),
            2 => "Stereo".to_string(),
            n => format!("{} channels", n),
        };

        lines.push(Line::from(vec![
            Span::styled("Channels: ", Style::default().fg(accent_color)),
            Span::raw(channel_str),
        ]));
    }

    let metadata_block = Block::default()
        .borders(Borders::ALL)
        .border_set(border_set)
        .border_style(Style::default().fg(accent_color))
        .title(" Metadata ");

    let metadata_widget = Paragraph::new(lines)
        .block(metadata_block)
        .wrap(Wrap { trim: true });

    if show_metadata_panel {
        f.render_widget(metadata_widget, center_layout[1]);
    }

    let progress = app.position.as_secs_f64() / track.duration.as_secs_f64();
    let progress_text = format!(
        "{} / {}",
        format_duration(app.position),
        format_duration(track.duration)
    );
    let progress_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border_set)
                .border_style(Style::default().fg(accent_color)),
        )
        .gauge_style(Style::default().fg(accent_color))
        .ratio(progress)
        .label(progress_text)
        .use_unicode(true);

    f.render_widget(progress_gauge, layout[2]);
}

fn scan_music_files(dir: &Path, recursive: bool) -> Result<Vec<Track>> {
    let mut tracks = Vec::new();
    let extensions = [
        "mp3",
        "aac",
        "wav",
        "flac",
        "alac",
        "aiff",
        "aif",
        "m4a"
    ];

    let walker = if recursive {
        WalkDir::new(dir).into_iter()
    } else {
        WalkDir::new(dir).max_depth(1).into_iter()
    };

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    let duration = get_audio_duration(path).unwrap_or(Duration::ZERO);
                    let metadata = Metadata::from_path(path);

                    tracks.push(Track {
                        path: path.to_path_buf(),
                        duration,
                        metadata,
                    });
                }
            }
        }
    }

    return Ok(tracks);
}

fn get_audio_duration(path: &Path) -> Result<Duration> {
    let file = std::fs::File::open(path)?;
    let source = Decoder::new(std::io::BufReader::new(file))?;

    return Ok(
        source
            .total_duration()
            .unwrap_or(Duration::ZERO)
    );
}

fn format_duration(d: Duration) -> String {
    return format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60);
}

fn toggle_playback(app: &mut App) {
    match app.playback_state {
        PlaybackState::Stopped => {
            play_track(app);
        }
        PlaybackState::Playing => {
            if let Some(sink) = &app.sink {
                sink.pause();
            }

            app.playback_state = PlaybackState::Paused;
            app.playback_start = None;
        }
        PlaybackState::Paused => {
            if let Some(sink) = &app.sink {
                sink.play();
            }

            app.playback_state = PlaybackState::Playing;
            app.playback_start = Some(Instant::now() - app.position);
        }
    }
}

fn seek(app: &mut App, seconds: i64) {
    let new_pos = app.position.as_secs() as i64 + seconds;
    let duration = app.tracks[app.current_track].duration.as_secs() as i64;
    let new_pos = new_pos.clamp(0, duration) as u64;

    app.position = Duration::from_secs(new_pos);

    if let (Some(sink), PlaybackState::Playing) = (&app.sink, &app.playback_state) {
        sink.stop();

        if let Ok(file) = std::fs::File::open(&app.tracks[app.current_track].path) {
            if let Ok(mut source) = Decoder::new(std::io::BufReader::new(file)) {
                source.try_seek(app.position).ok();
                sink.append(source);

                app.playback_start = Some(Instant::now() - app.position);
            }
        }
    } else if let Some(playback_start) = app.playback_start {
        app.playback_start = Some(playback_start);
    }
}

fn play_track(app: &mut App) {
    if let Ok((stream, handle)) = OutputStream::try_default() {
        if let Ok(file) = std::fs::File::open(&app.tracks[app.current_track].path) {
            if let Ok(source) = Decoder::new(std::io::BufReader::new(file)) {
                let sink = Sink::try_new(&handle).unwrap();

                sink.append(source);
                app.position = Duration::ZERO;
                app.playback_start = Some(Instant::now());
                app.sink = Some(sink);
                app._stream = Some(stream);
                app.playback_state = PlaybackState::Playing;

                return;
            }
        }
    }

    app.playback_state = PlaybackState::Stopped;
}

fn next_track(app: &mut App, direction: i32) {
    let len = app.tracks.len() as i32;

    app.current_track = (app.current_track as i32 + direction).rem_euclid(len) as usize;
    app.list_state.select(Some(app.current_track));
    app.position = Duration::ZERO;
    app.playback_start = None;

    if !matches!(app.playback_state, PlaybackState::Stopped) {
        play_track(app);
    }
}
