use lofty::{file::AudioFile, file::TaggedFileExt, read_from_path, tag::Accessor};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) struct Track {
    pub(crate) path: PathBuf,
    pub(crate) duration: Duration,
    pub(crate) metadata: Metadata,
}

#[derive(Default)]
pub(crate) struct Metadata {
    pub(crate) title: Option<String>,
    pub(crate) artist: Option<String>,
    pub(crate) album: Option<String>,
    pub(crate) genre: Option<String>,
    pub(crate) track_number: Option<u32>,
    pub(crate) release_year: Option<String>,
    pub(crate) duration: Option<Duration>,
    pub(crate) sample_rate: Option<u32>,
    pub(crate) bitrate: Option<u32>,
}

impl Metadata {
    pub(crate) fn from_path(path: &Path) -> Self {
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
