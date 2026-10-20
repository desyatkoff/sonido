use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Config {
    config: ConfigSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ConfigSettings {
    pub(crate) toggle_playback: Vec<String>,
    pub(crate) toggle_repeat: Vec<String>,
    pub(crate) seek_backward: Vec<String>,
    pub(crate) seek_forward: Vec<String>,
    pub(crate) seek_step: u64,
    pub(crate) previous_track: Vec<String>,
    pub(crate) next_track: Vec<String>,
    pub(crate) hide_track: Vec<String>,
    pub(crate) toggle_metadata_panel: Vec<String>,
    pub(crate) search: Vec<String>,
    pub(crate) reload_config: Vec<String>,
    pub(crate) quit: Vec<String>,
    pub(crate) show_app_title: bool,
    pub(crate) show_playlist_title: bool,
    pub(crate) show_playlist_scrollbar: bool,
    pub(crate) show_metadata_title: bool,
    pub(crate) show_metadata_panel: bool,
    pub(crate) show_progress_title: bool,
    pub(crate) show_search_title: bool,
    pub(crate) app_title_format: String,
    pub(crate) playlist_title_format: String,
    pub(crate) metadata_title_format: String,
    pub(crate) progress_title_format: String,
    pub(crate) search_title_format: String,
    pub(crate) app_title_alignment: String,
    pub(crate) playlist_title_alignment: String,
    pub(crate) metadata_title_alignment: String,
    pub(crate) progress_title_alignment: String,
    pub(crate) search_title_alignment: String,
    pub(crate) app_title_color: String,
    pub(crate) playlist_color: String,
    pub(crate) metadata_color: String,
    pub(crate) progress_color: String,
    pub(crate) search_color: String,
    pub(crate) rounded_corners: bool,
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

pub(crate) fn load_config() -> ConfigSettings {
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
