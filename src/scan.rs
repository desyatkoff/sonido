use directories::{BaseDirs, ProjectDirs};
use rodio::{Decoder, Source};
use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use walkdir::WalkDir;

use crate::{
    Args,
    app::App,
    metadata::{Metadata, Track},
};

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(stripped) = path.strip_prefix("~/")
        && let Some(base_dirs) = BaseDirs::new()
    {
        return base_dirs.home_dir().join(stripped);
    }

    PathBuf::from(path)
}

pub(crate) fn scan_tracks(app: Option<&App>, args: &Args) -> Result<Vec<Track>, Box<dyn Error>> {
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
