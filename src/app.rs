use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    widgets::{ListState, ScrollbarState},
};
use rodio::{Decoder, OutputStream, Sink, Source};
use std::{
    error::Error,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use crate::{
    Args,
    config::{ConfigSettings, load_config},
    keys::matches_key,
    metadata::Track,
    mpris::{self, Mpris, MprisCommand},
    scan::scan_tracks,
    ui::ui,
};

pub(crate) struct App {
    pub(crate) args: Args,
    pub(crate) tracks: Vec<Track>,
    pub(crate) config: ConfigSettings,
    pub(crate) current_track: usize,
    pub(crate) list_state: ListState,
    pub(crate) playback_state: PlaybackState,
    pub(crate) position: Duration,
    pub(crate) playback_start: Option<Instant>,
    pub(crate) repeat_mode: bool,
    pub(crate) sink: Option<Sink>,
    pub(crate) _stream: Option<OutputStream>,
    pub(crate) scroll_state: ScrollbarState,
    pub(crate) search_mode: bool,
    pub(crate) search_input: String,
    pub(crate) mpris: Option<Mpris>,
    pub(crate) mpris_commands: Receiver<MprisCommand>,
    pub(crate) mpris_art: Option<(PathBuf, Option<String>)>,
}

pub(crate) enum PlaybackState {
    Playing,
    Paused,
    Stopped,
}

impl App {
    pub(crate) fn new(args: Args, tracks: Vec<Track>, config: ConfigSettings) -> Self {
        let tracks_count = tracks.len();
        let (mpris_sender, mpris_commands) = mpsc::channel();

        let mut app = Self {
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

        app
    }
}

pub(crate) fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
) -> Result<(), Box<dyn Error>> {
    loop {
        if mpris::handle_mpris_commands(app) {
            return Ok(());
        }

        mpris::sync_mpris(app);

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

pub(crate) fn toggle_playback(app: &mut App) {
    match app.playback_state {
        PlaybackState::Playing => pause_playback(app),
        PlaybackState::Paused => resume_playback(app),
        PlaybackState::Stopped => play_track(app),
    }
}

pub(crate) fn pause_playback(app: &mut App) {
    if !matches!(app.playback_state, PlaybackState::Playing) {
        return;
    }

    if let Some(sink) = &app.sink {
        sink.pause();
    }

    app.playback_state = PlaybackState::Paused;
    app.playback_start = None;
}

pub(crate) fn resume_playback(app: &mut App) {
    if !matches!(app.playback_state, PlaybackState::Paused) {
        return;
    }

    if let Some(sink) = &app.sink {
        sink.play();
    }

    app.playback_state = PlaybackState::Playing;
    app.playback_start = Some(Instant::now() - app.position);
}

pub(crate) fn stop_playback(app: &mut App) {
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

pub(crate) fn seek_micros(app: &mut App, micros: i64) {
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

pub(crate) fn set_position(app: &mut App, position: Duration) {
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

pub(crate) fn play_track(app: &mut App) {
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

pub(crate) fn next_track(app: &mut App, direction: i32) {
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
