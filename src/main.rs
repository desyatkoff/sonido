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

mod app;
mod config;
mod keys;
mod metadata;
mod mpris;
mod scan;
mod ui;

use clap::Parser;
use colored::Colorize;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{env, error::Error, path::PathBuf};

use app::App;
use config::load_config;
use scan::scan_tracks;

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(
    name = "Sonido",
    about = "A sleek, terminal-based music player written in Rust",
    version
)]
pub(crate) struct Args {
    /// Scan path(s) from text file instead of specifying paths in arg
    #[arg(
        short = 'p',
        long = "playlist",
        num_args = 0..=1,
        default_missing_value = ""
    )]
    pub(crate) playlist: Option<String>,

    /// Scan path(s) recursively
    #[arg(short = 'r', long = "recursive")]
    pub(crate) recursive: bool,

    /// Sort tracks in alphabetical order
    #[arg(short = 's', long = "sort")]
    pub(crate) sort: bool,

    /// Which path(s) to scan for music
    #[arg(default_value = ".", conflicts_with = "playlist")]
    pub(crate) path: Vec<PathBuf>,
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
    let mut app = App::new(args, tracks, config);

    let result = app::run_app(&mut terminal, &mut app);

    disable_raw_mode()?;

    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}
