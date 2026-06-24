/*
 * This file is part of Sonido
 *
 * Copyright (C) 2026 Sergey Desyatkov
 *
 * Sonido is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published
 * by the Free Software Foundation, either version 3 of the License,
 * or (at your option) any later version
 *
 * Sonido is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details
 *
 * You should have received a copy of the GNU General Public License
 * along with Sonido. If not, see <https://www.gnu.org/licenses/>
 */

use clap::Parser;
use colored::Colorize;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use directories::{BaseDirs, ProjectDirs};
use lofty::{file::AudioFile, file::TaggedFileExt, read_from_path, tag::Accessor};
use ratatui::{
    Frame,
    prelude::*,
    symbols::border,
    widgets::{
        Block, Borders, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use rodio::{Decoder, OutputStream, Sink, Source};
use serde::{Deserialize, Serialize};
use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(
    name = "Sonido",
    about = "A sleek, terminal-based music player written in Rust",
    version
)]
struct Args {
    /// Scan path(s) from text file instead of specifying paths in arg
    #[arg(
        short = 'p',
        long = "playlist",
        num_args = 0..=1,
        default_missing_value = ""
    )]
    playlist: Option<String>,

    /// Scan path(s) recursively
    #[arg(short = 'r', long = "recursive")]
    recursive: bool,

    /// Sort tracks in alphabetical order
    #[arg(short = 's', long = "sort")]
    sort: bool,

    /// Which path(s) to scan for music
    #[arg(default_value = ".", conflicts_with = "playlist")]
    path: Vec<PathBuf>,
}

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
    genre: Option<String>,
    track_number: Option<u32>,
    release_year: Option<String>,
    duration: Option<Duration>,
    sample_rate: Option<u32>,
    bitrate: Option<u32>,
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
            let props = tagged_file.properties();

            if let Some(tag) = tag {
                metadata.title = tag.title().map(|s| s.to_string());
                metadata.artist = tag.artist().map(|s| s.to_string());
                metadata.album = tag.album().map(|s| s.to_string());
                metadata.genre = tag.genre().map(|s| s.to_string());
                metadata.track_number = tag.track();
                metadata.release_year = tag.year().map(|y| y.to_string());
                metadata.duration = Some(props.duration());
                metadata.sample_rate = props.sample_rate();
                metadata.bitrate = props.audio_bitrate();
            }
        }

        if metadata.title.is_none() {
            if let Some((artist, title)) = file_name.split_once(" - ") {
                metadata.title = Some(title.to_string());
                metadata.artist = Some(artist.to_string());
            } else {
                metadata.title = Some(file_name);
            }
        }

        metadata
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    config: ConfigSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ConfigSettings {
    toggle_playback: Vec<String>,
    toggle_repeat: Vec<String>,
    seek_backward: Vec<String>,
    seek_forward: Vec<String>,
    seek_step: u64,
    previous_track: Vec<String>,
    next_track: Vec<String>,
    hide_track: Vec<String>,
    toggle_metadata_panel: Vec<String>,
    reload_config: Vec<String>,
    quit: Vec<String>,
    show_app_title: bool,
    show_playlist_title: bool,
    show_playlist_scrollbar: bool,
    show_metadata_title: bool,
    show_metadata_panel: bool,
    show_progress_title: bool,
    app_title_format: String,
    playlist_title_format: String,
    metadata_title_format: String,
    progress_title_format: String,
    app_title_alignment: String,
    playlist_title_alignment: String,
    metadata_title_alignment: String,
    progress_title_alignment: String,
    app_title_color: String,
    playlist_color: String,
    metadata_color: String,
    progress_color: String,
    rounded_corners: bool,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        ConfigSettings {
            toggle_playback: vec!["space".to_string()],
            toggle_repeat: vec!["r".to_string()],
            seek_backward: vec!["h".to_string(), "left".to_string()],
            seek_forward: vec!["l".to_string(), "right".to_string()],
            seek_step: 5,
            previous_track: vec!["k".to_string(), "up".to_string()],
            next_track: vec!["j".to_string(), "down".to_string()],
            hide_track: vec!["x".to_string()],
            toggle_metadata_panel: vec!["m".to_string()],
            reload_config: vec!["c".to_string()],
            quit: vec!["q".to_string()],
            show_app_title: true,
            show_playlist_title: true,
            show_playlist_scrollbar: true,
            show_metadata_title: true,
            show_metadata_panel: true,
            show_progress_title: false,
            app_title_format: "┤ Sonido v{VERSION} ├".into(),
            playlist_title_format: "┤ Playlist ├".into(),
            metadata_title_format: "┤ Metadata ├".into(),
            progress_title_format: "┤ Progress ├".into(),
            app_title_alignment: "center".into(),
            playlist_title_alignment: "left".into(),
            metadata_title_alignment: "left".into(),
            progress_title_alignment: "left".into(),
            app_title_color: "blue".into(),
            metadata_color: "blue".into(),
            playlist_color: "blue".into(),
            progress_color: "blue".into(),
            rounded_corners: true,
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
    repeat_mode: bool,
    sink: Option<Sink>,
    _stream: Option<OutputStream>,
    scroll_state: ScrollbarState,
}

enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let tracks = if args.playlist.is_some() {
        let playlist_path = match args.playlist.as_deref() {
            Some("") => ProjectDirs::from("", "", "sonido")
                .unwrap()
                .config_dir()
                .join("playlist.txt"),
            Some(path) => PathBuf::from(path),
            None => PathBuf::new(),
        };

        scan_playlist_file(playlist_path, args.recursive, args.sort)?
    } else {
        scan_music_files(args.path, args.recursive, args.sort)?
    };

    if tracks.is_empty() {
        eprintln!(
            "{} could not find any music",
            Colorize::red("error:").bold(),
        );

        return Ok(());
    }

    enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let config = load_config();
    let tracks_count = tracks.len();

    let mut app = App {
        tracks,
        config,
        current_track: 0,
        list_state: ListState::default().with_selected(Some(0)),
        playback_state: PlaybackState::Stopped,
        position: Duration::ZERO,
        playback_start: None,
        repeat_mode: false,
        sink: None,
        _stream: None,
        scroll_state: ScrollbarState::new(tracks_count),
    };

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn parse_key(key_str: &str) -> KeyCode {
    match key_str.to_lowercase().as_str() {
        "space" => KeyCode::Char(' '),
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "escape" | "esc" => KeyCode::Esc,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "enter" => KeyCode::Enter,
        "insert" | "ins" => KeyCode::Insert,
        "delete" | "del" => KeyCode::Delete,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdown" => KeyCode::PageDown,
        key if key.len() == 1 => KeyCode::Char(key.chars().next().unwrap()),
        _ => KeyCode::Null,
    }
}

fn matches_key(key_code: KeyCode, bindings: &[String]) -> bool {
    bindings
        .iter()
        .any(|binding| parse_key(binding) == key_code)
}

fn parse_alignment(alignment_str: &str) -> Alignment {
    match alignment_str.to_lowercase().as_str() {
        "left" => Alignment::Left,
        "center" => Alignment::Center,
        "right" => Alignment::Right,
        _ => Alignment::Left,
    }
}

fn parse_color(color_str: &str) -> Color {
    match color_str.to_lowercase().as_str() {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "gray" | "grey" => Color::Gray,
        "darkgray" | "darkgrey" => Color::DarkGray,
        "lightred" => Color::LightRed,
        "lightgreen" => Color::LightGreen,
        "lightyellow" => Color::LightYellow,
        "lightblue" => Color::LightBlue,
        "lightmagenta" => Color::LightMagenta,
        "lightcyan" => Color::LightCyan,
        "white" => Color::White,
        _ => Color::Blue,
    }
}

fn load_config() -> ConfigSettings {
    if let Some(project_dirs) = ProjectDirs::from("", "", "sonido") {
        let config_directory = project_dirs.config_dir();
        let config_path = config_directory.join("config.toml");

        if let Ok(contents) = std::fs::read_to_string(&config_path) {
            match toml::from_str::<Config>(&contents) {
                Ok(config) => config.config,
                Err(_) => ConfigSettings::default(),
            }
        } else {
            let default_config = ConfigSettings::default();

            let _ = std::fs::create_dir_all(config_directory);

            if let Ok(toml_str) = toml::to_string(&Config {
                config: default_config.clone(),
            }) {
                let _ = std::fs::write(&config_path, toml_str);
            }

            default_config
        }
    } else {
        ConfigSettings::default()
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                _ if matches_key(key.code, &app.config.quit) => {
                    return Ok(());
                }
                _ if matches_key(key.code, &app.config.toggle_playback) => {
                    toggle_playback(app);
                }
                _ if matches_key(key.code, &app.config.toggle_repeat) => {
                    toggle_repeat(app);
                }
                _ if matches_key(key.code, &app.config.seek_backward) => {
                    seek(app, -(app.config.seek_step as i64));
                }
                _ if matches_key(key.code, &app.config.seek_forward) => {
                    seek(app, app.config.seek_step as i64);
                }
                _ if matches_key(key.code, &app.config.previous_track) => {
                    next_track(app, -1);
                }
                _ if matches_key(key.code, &app.config.next_track) => {
                    next_track(app, 1);
                }
                _ if matches_key(key.code, &app.config.hide_track) => {
                    hide_track(app, app.current_track);
                }
                _ if matches_key(key.code, &app.config.toggle_metadata_panel) => {
                    app.config.show_metadata_panel = !app.config.show_metadata_panel;
                }
                _ if matches_key(key.code, &app.config.reload_config) => {
                    app.config = load_config();
                }
                _ => {}
            }
        }

        if let (PlaybackState::Playing, Some(start_time)) =
            (&app.playback_state, app.playback_start)
        {
            app.position = start_time.elapsed();

            if app.position >= app.tracks[app.current_track].duration {
                next_track(app, !app.repeat_mode as i32);
            }
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let show_app_title = app.config.show_app_title;
    let show_playlist_title = app.config.show_playlist_title;
    let show_playlist_scrollbar = app.config.show_playlist_scrollbar;
    let show_metadata_title = app.config.show_metadata_title;
    let show_metadata_panel = app.config.show_metadata_panel;
    let show_progress_title = app.config.show_progress_title;

    let app_title_format = app
        .config
        .app_title_format
        .clone()
        .replace("{VERSION}", VERSION);
    let playlist_title_format = app.config.playlist_title_format.clone();
    let metadata_title_format = app.config.metadata_title_format.clone();
    let progress_title_format = app.config.progress_title_format.clone();

    let app_title_alignment = parse_alignment(&app.config.app_title_alignment);
    let playlist_title_alignment = parse_alignment(&app.config.playlist_title_alignment);
    let metadata_title_alignment = parse_alignment(&app.config.metadata_title_alignment);
    let progress_title_alignment = parse_alignment(&app.config.progress_title_alignment);

    let rounded_corners = app.config.rounded_corners;

    let app_title_color = parse_color(&app.config.app_title_color);
    let playlist_color = parse_color(&app.config.playlist_color);
    let metadata_color = parse_color(&app.config.metadata_color);
    let progress_color = parse_color(&app.config.progress_color);

    let mut list_state = app.list_state.clone();
    let track = &app.tracks[app.current_track];
    list_state.select(Some(app.current_track));

    let mut scrollbar_state = ScrollbarState::new(app.tracks.len()).position(app.current_track);

    let border_set = if rounded_corners {
        border::ROUNDED
    } else {
        border::PLAIN
    };

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(show_app_title.into()),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    let center_layout = if show_metadata_panel {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(layout[1])
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(layout[1])
    };

    let title = Block::default()
        .borders(Borders::TOP)
        .border_set(border_set)
        .border_style(Style::default().fg(app_title_color))
        .title(app_title_format)
        .title_alignment(app_title_alignment);

    f.render_widget(title, layout[0]);

    let items: Vec<ListItem> = app
        .tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let display_name = track.metadata.title.as_ref().cloned().unwrap_or_else(|| {
                track
                    .path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string()
            });

            let style = if i == app.current_track {
                Style::default().fg(playlist_color)
            } else {
                Style::default()
            };

            ListItem::new(display_name).style(style)
        })
        .collect();

    let list = if show_playlist_title {
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(border_set)
                    .border_style(Style::default().fg(playlist_color))
                    .title(playlist_title_format)
                    .title_alignment(playlist_title_alignment),
            )
            .highlight_style(Style::default().reversed())
    } else {
        List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(border_set)
                    .border_style(Style::default().fg(playlist_color)),
            )
            .highlight_style(Style::default().reversed())
    };

    f.render_stateful_widget(list, center_layout[0], &mut list_state);
    let scrollbar = Scrollbar::default()
        .orientation(ScrollbarOrientation::VerticalRight)
        .thumb_symbol("█")
        .track_symbol(None)
        .begin_symbol(Some("▲"))
        .end_symbol(Some("▼"))
        .style(Style::default().fg(playlist_color));

    if show_playlist_scrollbar {
        f.render_stateful_widget(
            scrollbar,
            Rect {
                x: center_layout[0].width.saturating_sub(1),
                y: center_layout[0].y.saturating_add(1),
                width: 1,
                height: center_layout[0].height.saturating_sub(2),
            },
            &mut scrollbar_state,
        );
    }

    let metadata = &track.metadata;

    let lines = vec![
        Line::from(vec![
            Span::styled("Title", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(metadata.title.as_deref().unwrap_or("Unknown")),
        ]),
        Line::from(vec![
            Span::styled("Artist", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(metadata.artist.as_deref().unwrap_or("Unknown")),
        ]),
        Line::from(vec![
            Span::styled("Album", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(metadata.album.as_deref().unwrap_or("Unknown")),
        ]),
        Line::from(vec![
            Span::styled("Genre", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(metadata.genre.as_deref().unwrap_or("Unknown")),
        ]),
        Line::from(vec![
            Span::styled("Track Number", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(
                metadata
                    .track_number
                    .map_or("Unknown".to_string(), |v| v.to_string()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Release Year", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(metadata.release_year.as_deref().unwrap_or("Unknown")),
        ]),
        Line::from(vec![
            Span::styled("Duration", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(
                metadata
                    .duration
                    .map_or("Unknown".to_string(), format_duration),
            ),
        ]),
        Line::from(vec![
            Span::styled("Sample Rate", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(
                metadata
                    .sample_rate
                    .map_or("Unknown".to_string(), |v| format!("{v} Hz")),
            ),
        ]),
        Line::from(vec![
            Span::styled("Bitrate", Style::default().fg(metadata_color)),
            Span::raw(": "),
            Span::raw(
                metadata
                    .bitrate
                    .map_or("Unknown".to_string(), |v| format!("{v} kbps")),
            ),
        ]),
    ];

    let metadata_block = if show_metadata_title {
        Block::default()
            .borders(Borders::ALL)
            .border_set(border_set)
            .border_style(Style::default().fg(metadata_color))
            .title(metadata_title_format)
            .title_alignment(metadata_title_alignment)
    } else {
        Block::default()
            .borders(Borders::ALL)
            .border_set(border_set)
            .border_style(Style::default().fg(metadata_color))
    };

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
    let progress_gauge = if show_progress_title {
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(border_set)
                    .border_style(Style::default().fg(progress_color))
                    .title(progress_title_format)
                    .title_alignment(progress_title_alignment),
            )
            .gauge_style(Style::default().fg(progress_color))
            .ratio(progress)
            .label(progress_text)
            .use_unicode(true)
    } else {
        Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(border_set)
                    .border_style(Style::default().fg(progress_color)),
            )
            .gauge_style(Style::default().fg(progress_color))
            .ratio(progress)
            .label(progress_text)
            .use_unicode(true)
    };

    f.render_widget(progress_gauge, layout[2]);
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(base_dirs) = BaseDirs::new()
    {
        return base_dirs.home_dir().join(stripped);
    }

    PathBuf::from(path)
}

fn scan_playlist_file(
    path_buf: PathBuf,
    recursive: bool,
    sort: bool,
) -> Result<Vec<Track>, Box<dyn Error>> {
    let content = fs::read_to_string(path_buf)?;
    let vec_path_buf: Vec<PathBuf> = content.lines().map(expand_tilde).collect();

    scan_music_files(vec_path_buf, recursive, sort)
}

fn scan_music_files(
    vec_path_buf: Vec<PathBuf>,
    recursive: bool,
    sort: bool,
) -> Result<Vec<Track>, Box<dyn Error>> {
    let mut tracks = Vec::new();
    let extensions = ["mp3", "aac", "wav", "flac", "alac", "aiff", "aif", "m4a"];

    for path_buf in vec_path_buf {
        if path_buf.is_dir() {
            let walker = if recursive {
                WalkDir::new(path_buf).into_iter()
            } else {
                WalkDir::new(path_buf).max_depth(1).into_iter()
            };

            for entry in walker.filter_map(|e| e.ok()) {
                let path = entry.path();

                if path.is_file()
                    && let Some(ext) = path.extension().and_then(|e| e.to_str())
                    && extensions.contains(&ext.to_lowercase().as_str())
                {
                    let duration = get_audio_duration(path).unwrap_or(Duration::ZERO);
                    let metadata = Metadata::from_path(path);

                    tracks.push(Track {
                        path: path.to_path_buf(),
                        duration,
                        metadata,
                    });
                }
            }
        } else if path_buf.is_file()
            && let Some(ext) = path_buf.extension().and_then(|e| e.to_str())
            && extensions.contains(&ext.to_lowercase().as_str())
        {
            let duration = get_audio_duration(&path_buf).unwrap_or(Duration::ZERO);
            let metadata = Metadata::from_path(&path_buf);

            tracks.push(Track {
                path: path_buf.to_path_buf(),
                duration,
                metadata,
            });
        }
    }

    if sort {
        tracks.sort_by(|a, b| {
            let a_title = a.metadata.title.as_deref().unwrap_or("").to_lowercase();
            let b_title = b.metadata.title.as_deref().unwrap_or("").to_lowercase();

            a_title.cmp(&b_title)
        });
    }

    Ok(tracks)
}

fn get_audio_duration(path: &Path) -> Result<Duration, Box<dyn Error>> {
    let file = std::fs::File::open(path)?;
    let source = Decoder::new(std::io::BufReader::new(file))?;

    Ok(source.total_duration().unwrap_or(Duration::ZERO))
}

fn format_duration(d: Duration) -> String {
    format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60)
}

fn toggle_playback(app: &mut App) {
    match app.playback_state {
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
        PlaybackState::Stopped => {
            play_track(app);
        }
    }
}

fn toggle_repeat(app: &mut App) {
    app.repeat_mode = !app.repeat_mode;
}

fn seek(app: &mut App, seconds: i64) {
    let new_pos = app.position.as_secs() as i64 + seconds;
    let duration = app.tracks[app.current_track].duration.as_secs() as i64;
    let new_pos = new_pos.clamp(0, duration) as u64;

    app.position = Duration::from_secs(new_pos);

    if let (Some(sink), PlaybackState::Playing) = (&app.sink, &app.playback_state) {
        sink.stop();

        if let Ok(file) = std::fs::File::open(&app.tracks[app.current_track].path)
            && let Ok(mut source) = Decoder::new(std::io::BufReader::new(file))
        {
            source.try_seek(app.position).ok();
            sink.append(source);

            app.playback_start = Some(Instant::now() - app.position);
        }
    } else if let Some(playback_start) = app.playback_start {
        app.playback_start = Some(playback_start);
    }
}

fn play_track(app: &mut App) {
    if let Ok((stream, handle)) = OutputStream::try_default()
        && let Ok(file) = std::fs::File::open(&app.tracks[app.current_track].path)
        && let Ok(source) = Decoder::new(std::io::BufReader::new(file))
    {
        let sink = Sink::try_new(&handle).unwrap();

        sink.append(source);
        app.position = Duration::ZERO;
        app.playback_start = Some(Instant::now());
        app.sink = Some(sink);
        app._stream = Some(stream);
        app.playback_state = PlaybackState::Playing;

        return;
    }

    app.playback_state = PlaybackState::Stopped;
}

fn next_track(app: &mut App, direction: i32) {
    let len = app.tracks.len() as i32;

    app.current_track = (app.current_track as i32 + direction).rem_euclid(len) as usize;
    app.list_state.select(Some(app.current_track));
    app.position = Duration::ZERO;
    app.playback_start = None;
    app.scroll_state = ScrollbarState::new(app.tracks.len()).position(app.current_track);

    if !matches!(app.playback_state, PlaybackState::Stopped) {
        play_track(app);
    }
}

fn hide_track(app: &mut App, index: usize) {
    app.tracks.remove(index);

    next_track(app, 0);
}
