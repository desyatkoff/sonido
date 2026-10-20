use ratatui::{
    Frame,
    prelude::*,
    symbols::border,
    widgets::{
        Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};
use std::time::Duration;

use crate::{VERSION, app::App, metadata::Metadata};

pub(crate) fn ui(f: &mut Frame, app: &mut App) {
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

fn format_duration(d: Duration) -> String {
    format!("{}:{:02}", d.as_secs() / 60, d.as_secs() % 60)
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
