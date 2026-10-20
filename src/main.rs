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
use lofty::{
    file::AudioFile, file::TaggedFileExt, picture::MimeType, read_from_path, tag::Accessor,
};
use mpris_server::{
    LoopStatus, Metadata as MprisMetadata, PlaybackStatus, PlayerInterface, Property,
    RootInterface, Server, Signal, Time, TrackId, zbus, zbus::fdo,
};
use ratatui::{
    Frame,
    prelude::*,
    symbols::border,
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use rodio::{Decoder, OutputStream, Sink, Source};
use serde::{Deserialize, Serialize};
use std::{
    collections::hash_map::DefaultHasher,
    env,
    error::Error,
    fs,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
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
    search: Vec<String>,
    reload_config: Vec<String>,
    quit: Vec<String>,
    show_app_title: bool,
    show_playlist_title: bool,
    show_playlist_scrollbar: bool,
    show_metadata_title: bool,
    show_metadata_panel: bool,
    show_progress_title: bool,
    show_search_title: bool,
    app_title_format: String,
    playlist_title_format: String,
    metadata_title_format: String,
    progress_title_format: String,
    search_title_format: String,
    app_title_alignment: String,
    playlist_title_alignment: String,
    metadata_title_alignment: String,
    progress_title_alignment: String,
    search_title_alignment: String,
    app_title_color: String,
    playlist_color: String,
    metadata_color: String,
    progress_color: String,
    search_color: String,
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
            search: vec!["/".to_string(), "s".to_string()],
            reload_config: vec!["c".to_string()],
            quit: vec!["q".to_string()],
            show_app_title: true,
            show_playlist_title: true,
            show_playlist_scrollbar: true,
            show_metadata_title: true,
            show_metadata_panel: true,
            show_progress_title: false,
            show_search_title: true,
            app_title_format: "┤ Sonido v{VERSION} ├".into(),
            playlist_title_format: "┤ Playlist │ {PLAYLIST_PROGRESS} ├".into(),
            metadata_title_format: "┤ Metadata ├".into(),
            progress_title_format: "┤ Progress ├".into(),
            search_title_format: "┤ Search ├".into(),
            app_title_alignment: "center".into(),
            playlist_title_alignment: "left".into(),
            metadata_title_alignment: "left".into(),
            progress_title_alignment: "left".into(),
            search_title_alignment: "left".into(),
            app_title_color: "blue".into(),
            metadata_color: "blue".into(),
            playlist_color: "blue".into(),
            progress_color: "blue".into(),
            search_color: "blue".into(),
            rounded_corners: true,
        }
    }
}

struct App {
    args: Args,
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
    search_mode: bool,
    search_input: String,
    mpris: Option<Mpris>,
    mpris_commands: Receiver<MprisCommand>,
    mpris_art: Option<(PathBuf, Option<String>)>,
}

enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

enum MprisCommand {
    Play,
    Pause,
    TogglePlayback,
    Stop,
    Next,
    Previous,
    SeekBy(i64),
    SetPosition(TrackId, Time),
    SetLoopStatus(LoopStatus),
    SetVolume(f64),
    Quit,
}

struct MprisState {
    metadata: MprisMetadata,
    track_id: TrackId,
    playback_status: PlaybackStatus,
    loop_status: LoopStatus,
    position: Duration,
    volume: f64,
    has_tracks: bool,
}

struct MprisPlayer {
    state: Arc<Mutex<MprisState>>,
    commands: Sender<MprisCommand>,
}

impl MprisPlayer {
    fn state(&self) -> std::sync::MutexGuard<'_, MprisState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn send(&self, command: MprisCommand) {
        let _ = self.commands.send(command);
    }
}

struct MprisSnapshot {
    metadata: MprisMetadata,
    track_id: TrackId,
    playback_status: PlaybackStatus,
    loop_status: LoopStatus,
    position: Duration,
    has_tracks: bool,
}

struct Mpris {
    server: Server<MprisPlayer>,
    state: Arc<Mutex<MprisState>>,
}

impl Mpris {
    fn new(commands: Sender<MprisCommand>) -> zbus::Result<Self> {
        let state = Arc::new(Mutex::new(MprisState {
            metadata: MprisMetadata::new(),
            track_id: TrackId::NO_TRACK,
            playback_status: PlaybackStatus::Stopped,
            loop_status: LoopStatus::None,
            position: Duration::ZERO,
            volume: 1.0,
            has_tracks: false,
        }));

        let build = |suffix: &str| {
            let player = MprisPlayer {
                state: Arc::clone(&state),
                commands: commands.clone(),
            };

            zbus::block_on(Server::new(suffix, player))
        };

        let server = build("sonido")
            .or_else(|_| build(&format!("sonido.instance{}", std::process::id())))?;

        Ok(Self { server, state })
    }

    fn sync(&self, snapshot: MprisSnapshot) {
        let mut changed = Vec::new();

        {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            state.position = snapshot.position;

            if state.track_id != snapshot.track_id || state.metadata != snapshot.metadata {
                state.track_id = snapshot.track_id;
                state.metadata = snapshot.metadata.clone();
                changed.push(Property::Metadata(snapshot.metadata));
            }

            if state.playback_status != snapshot.playback_status {
                state.playback_status = snapshot.playback_status;
                changed.push(Property::PlaybackStatus(snapshot.playback_status));
            }

            if state.loop_status != snapshot.loop_status {
                state.loop_status = snapshot.loop_status;
                changed.push(Property::LoopStatus(snapshot.loop_status));
            }

            if state.has_tracks != snapshot.has_tracks {
                state.has_tracks = snapshot.has_tracks;
                changed.push(Property::CanGoNext(snapshot.has_tracks));
                changed.push(Property::CanGoPrevious(snapshot.has_tracks));
                changed.push(Property::CanPlay(snapshot.has_tracks));
                changed.push(Property::CanPause(snapshot.has_tracks));
                changed.push(Property::CanSeek(snapshot.has_tracks));
            }
        }

        if !changed.is_empty() {
            let _ = zbus::block_on(self.server.properties_changed(changed));
        }
    }

    fn seeked(&self, position: Duration) {
        let _ = zbus::block_on(self.server.emit(Signal::Seeked {
            position: Time::from_micros(position.as_micros() as i64),
        }));
    }

    fn track_id(&self) -> TrackId {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .track_id
            .clone()
    }

    fn volume(&self) -> f64 {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .volume
    }

    fn set_volume(&self, volume: f64) {
        let changed = {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            if state.volume == volume {
                false
            } else {
                state.volume = volume;
                true
            }
        };

        if changed {
            let _ = zbus::block_on(self.server.properties_changed([Property::Volume(volume)]));
        }
    }
}

impl RootInterface for MprisPlayer {
    async fn raise(&self) -> fdo::Result<()> {
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        self.send(MprisCommand::Quit);

        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _fullscreen: bool) -> zbus::Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok("Sonido".to_string())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("sonido".to_string())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["file".to_string()])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(vec![
            "audio/aac".to_string(),
            "audio/aiff".to_string(),
            "audio/flac".to_string(),
            "audio/mp4".to_string(),
            "audio/mpeg".to_string(),
            "audio/wav".to_string(),
        ])
    }
}

impl PlayerInterface for MprisPlayer {
    async fn next(&self) -> fdo::Result<()> {
        self.send(MprisCommand::Next);

        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.send(MprisCommand::Previous);

        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.send(MprisCommand::Pause);

        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.send(MprisCommand::TogglePlayback);

        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.send(MprisCommand::Stop);

        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.send(MprisCommand::Play);

        Ok(())
    }

    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        self.send(MprisCommand::SeekBy(offset.as_micros()));

        Ok(())
    }

    async fn set_position(&self, track_id: TrackId, position: Time) -> fdo::Result<()> {
        self.send(MprisCommand::SetPosition(track_id, position));

        Ok(())
    }

    async fn open_uri(&self, _uri: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported(
            "Opening URIs is not supported".to_string(),
        ))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(self.state().playback_status)
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(self.state().loop_status)
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> zbus::Result<()> {
        self.send(MprisCommand::SetLoopStatus(loop_status));

        Ok(())
    }

    async fn rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn set_rate(&self, _rate: f64) -> zbus::Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_shuffle(&self, _shuffle: bool) -> zbus::Result<()> {
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<MprisMetadata> {
        Ok(self.state().metadata.clone())
    }

    async fn volume(&self) -> fdo::Result<f64> {
        Ok(self.state().volume)
    }

    async fn set_volume(&self, volume: f64) -> zbus::Result<()> {
        self.send(MprisCommand::SetVolume(volume));

        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(Time::from_micros(self.state().position.as_micros() as i64))
    }

    async fn minimum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<f64> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(self.state().has_tracks)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(self.state().has_tracks)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(self.state().has_tracks)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(self.state().has_tracks)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(self.state().has_tracks)
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let tracks = scan_tracks(None, &args)?;

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
    let (mpris_sender, mpris_commands) = mpsc::channel();

    let mut app = App {
        args,
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
        search_mode: false,
        search_input: String::new(),
        mpris: None,
        mpris_commands,
        mpris_art: None,
    };

    app.mpris = Mpris::new(mpris_sender).ok();

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn parse_key(key_str: &str) -> KeyCode {
    match key_str.to_lowercase().as_str() {
        "space" | " " => KeyCode::Char(' '),
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
        "slash" | "/" => KeyCode::Char('/'),
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
        if handle_mpris_commands(app) {
            return Ok(());
        }

        sync_mpris(app);

        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            if !app.search_mode {
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
                    _ if matches_key(key.code, &app.config.search) => {
                        app.search_mode = true;
                        app.search_input = String::new();
                    }
                    _ if matches_key(key.code, &app.config.reload_config) => {
                        app.config = load_config();
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc => {
                        app.search_input = String::new();
                        app.search_mode = false;
                    }
                    KeyCode::Enter => {
                        app.current_track = 0;
                        app.playback_state = PlaybackState::Stopped;
                        app.list_state = ListState::default().with_selected(Some(0));
                        app.position = Duration::ZERO;
                        app.playback_start = None;
                        app.sink = None;
                        app._stream = None;
                        app.scroll_state = ScrollbarState::new(0);

                        app.tracks = scan_tracks(Some(app), &app.args)?;
                        app.search_mode = false;
                    }
                    KeyCode::Backspace => {
                        app.search_input.pop();
                    }
                    KeyCode::Char(c) => {
                        app.search_input.push(c);
                    }
                    _ => {}
                }
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

fn ui(f: &mut Frame, app: &mut App) {
    let show_app_title = app.config.show_app_title;
    let show_playlist_title = app.config.show_playlist_title;
    let show_playlist_scrollbar = app.config.show_playlist_scrollbar;
    let show_metadata_title = app.config.show_metadata_title;
    let show_metadata_panel = app.config.show_metadata_panel;
    let show_progress_title = app.config.show_progress_title;
    let show_search_title = app.config.show_search_title;

    let app_title_format = app
        .config
        .app_title_format
        .clone()
        .replace("{VERSION}", VERSION);
    let playlist_title_format = app.config.playlist_title_format.clone().replace(
        "{PLAYLIST_PROGRESS}",
        &format!("{} / {}", app.current_track + 1, app.tracks.len()),
    );
    let metadata_title_format = app.config.metadata_title_format.clone();
    let progress_title_format = app.config.progress_title_format.clone();
    let search_title_format = app.config.search_title_format.clone();

    let app_title_alignment = parse_alignment(&app.config.app_title_alignment);
    let playlist_title_alignment = parse_alignment(&app.config.playlist_title_alignment);
    let metadata_title_alignment = parse_alignment(&app.config.metadata_title_alignment);
    let progress_title_alignment = parse_alignment(&app.config.progress_title_alignment);
    let search_title_alignment = parse_alignment(&app.config.search_title_alignment);

    let rounded_corners = app.config.rounded_corners;

    let app_title_color = parse_color(&app.config.app_title_color);
    let playlist_color = parse_color(&app.config.playlist_color);
    let metadata_color = parse_color(&app.config.metadata_color);
    let progress_color = parse_color(&app.config.progress_color);
    let search_color = parse_color(&app.config.search_color);

    let mut list_state = app.list_state.clone();

    if app.tracks.is_empty() {
        app.current_track = 0;
        list_state.select(None);
    } else {
        app.current_track = app.current_track.min(app.tracks.len() - 1);
        list_state.select(Some(app.current_track));
    }

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

    let default_metadata = Metadata::default();

    let metadata = app
        .tracks
        .get(app.current_track)
        .map(|track| &track.metadata)
        .unwrap_or(&default_metadata);

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

    let progress: f64;
    let progress_text: String;

    if let Some(track) = app.tracks.get(app.current_track) {
        progress = app.position.as_secs_f64() / track.duration.as_secs_f64();

        progress_text = format!(
            "{} / {}",
            format_duration(app.position),
            format_duration(track.duration)
        );
    } else {
        progress = 0.0;

        progress_text = "0:00 / 0:00".to_string();
    }

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

    if app.search_mode {
        let area = centered_rect(50, 1, f.area());

        f.render_widget(Clear, area);

        let block = if show_search_title {
            Block::default()
                .borders(Borders::ALL)
                .border_set(border_set)
                .border_style(Style::default().fg(search_color))
                .title(search_title_format)
                .title_alignment(search_title_alignment)
        } else {
            Block::default()
                .borders(Borders::ALL)
                .border_set(border_set)
                .border_style(Style::default().fg(search_color))
        };

        let input = Paragraph::new(app.search_input.as_str()).block(block);

        f.render_widget(input, area);

        f.set_cursor_position(Position {
            x: area.x + app.search_input.len() as u16 + 1,
            y: area.y + 1,
        });
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_w = (r.width as u32 * percent_x as u32 / 100) as u16;
    let popup_h = (r.height as u32 * percent_y as u32 / 100) as u16;

    let x = r.x + (r.width.saturating_sub(popup_w)) / 2;
    let y = r.y + (r.height.saturating_sub(popup_h)) / 2;

    Rect::new(
        x,
        y,
        popup_w.max(10).min(r.width),
        popup_h.max(3).min(r.height),
    )
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(base_dirs) = BaseDirs::new()
    {
        return base_dirs.home_dir().join(stripped);
    }

    PathBuf::from(path)
}

fn scan_tracks(app: Option<&App>, args: &Args) -> Result<Vec<Track>, Box<dyn Error>> {
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
        scan_music_files(&args.path, args.recursive, args.sort)?
    };

    if app.is_some() {
        let app = app.unwrap();

        return Ok(tracks
            .into_iter()
            .filter(|t| {
                if app.search_input.is_empty() {
                    return true;
                }

                t.metadata
                    .title
                    .as_ref()
                    .cloned()
                    .unwrap_or_else(|| {
                        t.path
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Unknown")
                            .to_string()
                    })
                    .to_lowercase()
                    .contains(&app.search_input.to_lowercase())
            })
            .collect());
    }

    Ok(tracks)
}

fn scan_playlist_file(
    path_buf: PathBuf,
    recursive: bool,
    sort: bool,
) -> Result<Vec<Track>, Box<dyn Error>> {
    let content = fs::read_to_string(path_buf)?;
    let vec_path_buf: Vec<PathBuf> = content.lines().map(expand_tilde).collect();

    scan_music_files(&vec_path_buf, recursive, sort)
}

fn scan_music_files(
    vec_path_buf: &Vec<PathBuf>,
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

fn mpris_metadata(
    track: Option<&Track>,
    index: usize,
    art_url: Option<&str>,
) -> (MprisMetadata, TrackId) {
    let Some(track) = track else {
        return (MprisMetadata::new(), TrackId::NO_TRACK);
    };

    let track_id =
        TrackId::try_from(format!("/org/sonido/track/{index}")).unwrap_or(TrackId::NO_TRACK);

    let mut builder = MprisMetadata::builder().trackid(track_id.clone());

    if !track.duration.is_zero() {
        builder = builder.length(Time::from_micros(track.duration.as_micros() as i64));
    }

    if let Some(art_url) = art_url {
        builder = builder.art_url(art_url);
    }

    if let Some(title) = &track.metadata.title {
        builder = builder.title(title);
    }

    if let Some(artist) = &track.metadata.artist {
        builder = builder.artist([artist.as_str()]);
    }

    if let Some(album) = &track.metadata.album {
        builder = builder.album(album);
    }

    if let Some(genre) = &track.metadata.genre {
        builder = builder.genre([genre.as_str()]);
    }

    if let Some(track_number) = track.metadata.track_number {
        builder = builder.track_number(track_number as i32);
    }

    (builder.build(), track_id)
}

fn cover_art_url(track_path: &Path) -> Option<String> {
    if let Ok(tagged_file) = read_from_path(track_path) {
        let tag = tagged_file
            .primary_tag()
            .or_else(|| tagged_file.first_tag());

        if let Some(picture) = tag.and_then(|tag| tag.pictures().first()) {
            let data = picture.data();

            if !data.is_empty() && data.len() <= 16 * 1024 * 1024 {
                let extension = match picture.mime_type() {
                    Some(MimeType::Png) => "png",
                    Some(MimeType::Tiff) => "tiff",
                    Some(MimeType::Bmp) => "bmp",
                    Some(MimeType::Gif) => "gif",
                    Some(MimeType::Jpeg) | None => "jpg",
                    Some(MimeType::Unknown(mime)) => mime
                        .rsplit('/')
                        .next()
                        .filter(|ext| {
                            !ext.is_empty() && ext.chars().all(|c| c.is_ascii_alphanumeric())
                        })
                        .unwrap_or("jpg"),
                    Some(_) => "jpg",
                };

                return cache_cover_art(track_path, data, extension);
            }
        }
    }

    folder_cover_art(track_path)
}

fn cache_cover_art(track_path: &Path, data: &[u8], extension: &str) -> Option<String> {
    let cache_dir = ProjectDirs::from("", "", "sonido")?
        .cache_dir()
        .join("artwork");

    fs::create_dir_all(&cache_dir).ok()?;

    let mut hasher = DefaultHasher::new();

    track_path.hash(&mut hasher);

    if let Ok(metadata) = fs::metadata(track_path) {
        metadata.len().hash(&mut hasher);

        if let Ok(modified) = metadata.modified() {
            modified.hash(&mut hasher);
        }
    }

    let cache_path = cache_dir.join(format!("{:016x}.{extension}", hasher.finish()));

    if !cache_path.is_file() {
        fs::write(&cache_path, data).ok()?;
    }

    let cache_path = fs::canonicalize(cache_path).ok()?;

    Some(path_to_file_url(&cache_path))
}

fn folder_cover_art(track_path: &Path) -> Option<String> {
    let directory = track_path.parent()?;
    let names = ["cover", "folder", "front", "album", "albumart", "artwork"];
    let extensions = ["jpg", "jpeg", "png", "webp", "bmp", "gif"];

    for entry in fs::read_dir(directory).ok()?.flatten() {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_lowercase();

        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_lowercase();

        if names.contains(&name.as_str()) && extensions.contains(&extension.as_str()) {
            let path = fs::canonicalize(path).ok()?;

            return Some(path_to_file_url(&path));
        }
    }

    None
}

fn path_to_file_url(path: &Path) -> String {
    let mut url = String::from("file://");

    for byte in path.to_string_lossy().bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                url.push(byte as char);
            }
            _ => url.push_str(&format!("%{byte:02X}")),
        }
    }

    url
}

fn sync_mpris(app: &mut App) {
    if app.mpris.is_none() {
        return;
    }

    let current_path = app
        .tracks
        .get(app.current_track)
        .map(|track| track.path.clone());

    let art_url = match &current_path {
        Some(path) => {
            if app.mpris_art.as_ref().map(|(cached, _)| cached) != Some(path) {
                let url = cover_art_url(path);
                app.mpris_art = Some((path.clone(), url));
            }

            app.mpris_art.as_ref().and_then(|(_, url)| url.as_deref())
        }
        None => {
            app.mpris_art = None;
            None
        }
    };

    let Some(mpris) = &app.mpris else {
        return;
    };

    let (metadata, track_id) = mpris_metadata(
        app.tracks.get(app.current_track),
        app.current_track,
        art_url,
    );

    mpris.sync(MprisSnapshot {
        metadata,
        track_id,
        playback_status: match app.playback_state {
            PlaybackState::Playing => PlaybackStatus::Playing,
            PlaybackState::Paused => PlaybackStatus::Paused,
            PlaybackState::Stopped => PlaybackStatus::Stopped,
        },
        loop_status: if app.repeat_mode {
            LoopStatus::Track
        } else {
            LoopStatus::None
        },
        position: app.position,
        has_tracks: !app.tracks.is_empty(),
    });
}

fn handle_mpris_commands(app: &mut App) -> bool {
    while let Ok(command) = app.mpris_commands.try_recv() {
        match command {
            MprisCommand::Play => match app.playback_state {
                PlaybackState::Playing => {}
                PlaybackState::Paused => resume_playback(app),
                PlaybackState::Stopped => play_track(app),
            },
            MprisCommand::Pause => pause_playback(app),
            MprisCommand::TogglePlayback => toggle_playback(app),
            MprisCommand::Stop => stop_playback(app),
            MprisCommand::Next => next_track(app, 1),
            MprisCommand::Previous => next_track(app, -1),
            MprisCommand::SeekBy(micros) => seek_micros(app, micros),
            MprisCommand::SetPosition(track_id, position) => {
                let is_current = app
                    .mpris
                    .as_ref()
                    .is_some_and(|mpris| mpris.track_id() == track_id);

                if is_current {
                    set_position(
                        app,
                        Duration::from_micros(position.as_micros().max(0) as u64),
                    );
                }
            }
            MprisCommand::SetLoopStatus(loop_status) => {
                app.repeat_mode = !matches!(loop_status, LoopStatus::None);
            }
            MprisCommand::SetVolume(volume) => {
                let volume = volume.clamp(0.0, 1.0);

                if let Some(sink) = &app.sink {
                    sink.set_volume(volume as f32);
                }

                if let Some(mpris) = &app.mpris {
                    mpris.set_volume(volume);
                }
            }
            MprisCommand::Quit => return true,
        }
    }

    false
}

fn toggle_playback(app: &mut App) {
    match app.playback_state {
        PlaybackState::Playing => pause_playback(app),
        PlaybackState::Paused => resume_playback(app),
        PlaybackState::Stopped => play_track(app),
    }
}

fn pause_playback(app: &mut App) {
    if !matches!(app.playback_state, PlaybackState::Playing) {
        return;
    }

    if let Some(sink) = &app.sink {
        sink.pause();
    }

    app.playback_state = PlaybackState::Paused;
    app.playback_start = None;
}

fn resume_playback(app: &mut App) {
    if !matches!(app.playback_state, PlaybackState::Paused) {
        return;
    }

    if let Some(sink) = &app.sink {
        sink.play();
    }

    app.playback_state = PlaybackState::Playing;
    app.playback_start = Some(Instant::now() - app.position);
}

fn stop_playback(app: &mut App) {
    if let Some(sink) = &app.sink {
        sink.stop();
    }

    app.sink = None;
    app._stream = None;
    app.playback_state = PlaybackState::Stopped;
    app.position = Duration::ZERO;
    app.playback_start = None;
}

fn toggle_repeat(app: &mut App) {
    app.repeat_mode = !app.repeat_mode;
}

fn seek(app: &mut App, seconds: i64) {
    if app.tracks.is_empty() {
        return;
    }

    let offset = Duration::from_secs(seconds.unsigned_abs());

    let position = if seconds < 0 {
        app.position.saturating_sub(offset)
    } else {
        app.position.saturating_add(offset)
    };

    set_position(app, position);
}

fn seek_micros(app: &mut App, micros: i64) {
    if app.tracks.is_empty() {
        return;
    }

    let offset = Duration::from_micros(micros.unsigned_abs());

    let position = if micros < 0 {
        app.position.saturating_sub(offset)
    } else {
        app.position.saturating_add(offset)
    };

    set_position(app, position);
}

fn set_position(app: &mut App, position: Duration) {
    let Some(track) = app.tracks.get(app.current_track) else {
        return;
    };

    let position = position.min(track.duration);

    app.position = position;

    match app.playback_state {
        PlaybackState::Playing => restart_sink_at(app, position, true),
        PlaybackState::Paused => restart_sink_at(app, position, false),
        PlaybackState::Stopped => {}
    }

    if let Some(mpris) = &app.mpris {
        mpris.seeked(position);
    }
}

fn restart_sink_at(app: &mut App, position: Duration, playing: bool) {
    let (Some(sink), Some(track)) = (&app.sink, app.tracks.get(app.current_track)) else {
        return;
    };

    sink.stop();

    if let Ok(file) = std::fs::File::open(&track.path)
        && let Ok(mut source) = Decoder::new(std::io::BufReader::new(file))
    {
        source.try_seek(position).ok();
        sink.append(source);

        if playing {
            sink.play();

            app.playback_start = Some(Instant::now() - position);
        } else {
            sink.pause();

            app.playback_start = None;
        }
    }
}

fn play_track(app: &mut App) {
    let len = app.tracks.len() as i32;

    if len < 1 {
        return;
    }

    if let Ok((stream, handle)) = OutputStream::try_default()
        && let Ok(file) = std::fs::File::open(&app.tracks[app.current_track].path)
        && let Ok(mut source) = Decoder::new(std::io::BufReader::new(file))
    {
        let sink = Sink::try_new(&handle).unwrap();

        if !app.position.is_zero() {
            source.try_seek(app.position).ok();
        }

        if let Some(mpris) = &app.mpris {
            sink.set_volume(mpris.volume() as f32);
        }

        sink.append(source);

        app.playback_start = Some(Instant::now() - app.position);
        app.sink = Some(sink);
        app._stream = Some(stream);
        app.playback_state = PlaybackState::Playing;

        return;
    }

    app.playback_state = PlaybackState::Stopped;
}

fn next_track(app: &mut App, direction: i32) {
    let len = app.tracks.len() as i32;

    if len < 1 {
        return;
    }

    let was_paused = matches!(app.playback_state, PlaybackState::Paused);

    app.current_track = (app.current_track as i32 + direction).rem_euclid(len) as usize;
    app.list_state.select(Some(app.current_track));
    app.position = Duration::ZERO;
    app.playback_start = None;
    app.scroll_state = ScrollbarState::new(app.tracks.len()).position(app.current_track);

    if !matches!(app.playback_state, PlaybackState::Stopped) {
        play_track(app);

        if was_paused {
            pause_playback(app);
        }
    }
}

fn hide_track(app: &mut App, index: usize) {
    let len = app.tracks.len() as i32;

    if len < 1 {
        return;
    }

    app.tracks.remove(index);

    next_track(app, 0);
}
