/*
Copyright (C) 2025 Desyatkov Sergey
This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.
*/

use std::{
    env, fs,
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
use ratatui::{
    prelude::*,
    symbols::border,
    widgets::{
        Block,
        Borders,
        List,
        ListItem,
        ListState,
        Paragraph,
    },
    Frame,
};
use rodio::{
    Decoder,
    OutputStream,
    Sink,
    Source,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");

struct Track {
    path: PathBuf,
    duration: Duration,
}

struct App {
    tracks: Vec<Track>,
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
    let music_directory = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().unwrap());

    let tracks = scan_music_files(&music_directory)?;

    if tracks.is_empty() {
        anyhow::bail!("No music files found in {}", music_directory.display());
    }

    enable_raw_mode()?;

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App {
        tracks,
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

fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => {
                            return Ok(());
                        },
                        KeyCode::Char(' ') => {
                            toggle_playback(app);
                        },
                        KeyCode::Left => {
                            seek(app, -5);
                        },
                        KeyCode::Right => {
                            seek(app, 5);
                        },
                        KeyCode::Up => {
                            next_track(app, -1);
                        },
                        KeyCode::Down => {
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
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Block::default()
        .borders(Borders::ALL)
        .border_set(border::ROUNDED)
        .border_style(Style::default().fg(Color::Blue))
        .title(format!(" Sonido v{} ", VERSION))
        .title_alignment(Alignment::Center);

    f.render_widget(title, layout[0]);

    let items: Vec<ListItem> = app
        .tracks
        .iter()
        .enumerate()
        .map(|(i, track)| {
            let name = track
                .path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");

            let style = if i == app.current_track {
                Style::default().fg(Color::Blue)
            } else {
                Style::default().fg(Color::White)
            };

            return ListItem::new(name).style(style);
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .highlight_style(Style::default().bold());

    f.render_stateful_widget(list, layout[1], &mut app.list_state.clone());

    let track = &app.tracks[app.current_track];
    let progress = format!(
        "{} / {}",
        format_duration(app.position),
        format_duration(track.duration)
    );

    let progress_bar = Paragraph::new(progress)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_set(border::ROUNDED)
                .border_style(Style::default().fg(Color::Blue)),
        )
        .alignment(Alignment::Center);

    f.render_widget(progress_bar, layout[2]);
}

fn scan_music_files(dir: &Path) -> Result<Vec<Track>> {
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

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    let duration = get_audio_duration(&path)?;

                    tracks.push(Track { path, duration });
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
