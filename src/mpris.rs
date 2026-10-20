use directories::ProjectDirs;
use lofty::{file::TaggedFileExt, picture::MimeType, read_from_path};
use mpris_server::{
    LoopStatus, Metadata as MprisMetadata, PlaybackStatus, PlayerInterface, Property,
    RootInterface, Server, Signal, Time, TrackId, zbus, zbus::fdo,
};
use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    path::Path,
    sync::{Arc, Mutex, mpsc::Sender},
    time::Duration,
};

use crate::{
    app::{
        App, PlaybackState, next_track, pause_playback, play_track, resume_playback, seek_micros,
        set_position, stop_playback, toggle_playback,
    },
    metadata::Track,
};

pub(crate) enum MprisCommand {
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

pub(crate) struct Mpris {
    server: Server<MprisPlayer>,
    state: Arc<Mutex<MprisState>>,
}

impl Mpris {
    pub(crate) fn new(commands: Sender<MprisCommand>) -> zbus::Result<Self> {
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

    pub(crate) fn seeked(&self, position: Duration) {
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

    pub(crate) fn volume(&self) -> f64 {
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

pub(crate) fn sync_mpris(app: &mut App) {
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

pub(crate) fn handle_mpris_commands(app: &mut App) -> bool {
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
