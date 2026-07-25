use std::io::{self, Stdout, Write};

use crossterm::{
    QueueableCommand,
    cursor::{Hide, MoveTo, Show},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{
        self, BeginSynchronizedUpdate, Clear, ClearType, DisableLineWrap, EnableLineWrap,
        EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
    },
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    app::{
        App, DetailView, FolderFocus, PlaybackState, ScanPhase, SearchItem, Theme, View,
    },
    library::{DirectoryEntryKind, Track},
};

#[derive(Clone, Copy)]
struct Palette {
    background: Color,
    sidebar: Color,
    surface: Color,
    surface_alt: Color,
    player: Color,
    text: Color,
    muted: Color,
    faint: Color,
    accent: Color,
    selected: Color,
    current: Color,
    border: Color,
}

impl Palette {
    fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                background: rgb(18, 18, 18),
                sidebar: rgb(0, 0, 0),
                surface: rgb(24, 24, 24),
                surface_alt: rgb(32, 32, 32),
                player: rgb(20, 20, 20),
                text: rgb(255, 255, 255),
                muted: rgb(179, 179, 179),
                faint: rgb(105, 105, 105),
                accent: rgb(30, 215, 96),
                selected: rgb(52, 52, 52),
                current: rgb(31, 55, 39),
                border: rgb(64, 64, 64),
            },
            Theme::Light => Self {
                background: rgb(245, 245, 245),
                sidebar: rgb(232, 232, 232),
                surface: rgb(255, 255, 255),
                surface_alt: rgb(238, 238, 238),
                player: rgb(250, 250, 250),
                text: rgb(20, 20, 20),
                muted: rgb(86, 86, 86),
                faint: rgb(145, 145, 145),
                accent: rgb(24, 174, 80),
                selected: rgb(220, 220, 220),
                current: rgb(214, 239, 222),
                border: rgb(205, 205, 205),
            },
        }
    }
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb { r, g, b }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Style {
    fg: Color,
    bg: Color,
    bold: bool,
}

impl Style {
    const fn new(fg: Color, bg: Color) -> Self {
        Self { fg, bg, bold: false }
    }

    const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

#[derive(Clone, Copy)]
struct Cell {
    ch: char,
    style: Style,
}

struct Canvas {
    width: u16,
    height: u16,
    cells: Vec<Cell>,
}

impl Canvas {
    fn new(width: u16, height: u16, palette: Palette) -> Self {
        let cell = Cell {
            ch: ' ',
            style: Style::new(palette.text, palette.background),
        };
        Self {
            width,
            height,
            cells: vec![cell; width as usize * height as usize],
        }
    }

    fn put(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.cells[y as usize * self.width as usize + x as usize] = Cell { ch, style };
    }

    fn text(&mut self, x: u16, y: u16, text: &str, max_width: u16, style: Style) {
        if y >= self.height || x >= self.width || max_width == 0 {
            return;
        }

        let right = x.saturating_add(max_width).min(self.width);
        let mut column = x;
        for ch in text.chars() {
            if ch.is_control() {
                continue;
            }
            let width = ch.width().unwrap_or(0) as u16;
            if width == 0 {
                continue;
            }
            if column.saturating_add(width) > right {
                break;
            }

            self.put(column, y, ch, style);
            for continuation in 1..width {
                self.put(column + continuation, y, '\0', style);
            }
            column += width;
        }
    }

    fn text_right(&mut self, right: u16, y: u16, text: &str, max_width: u16, style: Style) {
        let width = UnicodeWidthStr::width(text).min(max_width as usize) as u16;
        self.text(right.saturating_sub(width), y, text, max_width, style);
    }

    fn text_center(&mut self, x: u16, y: u16, width: u16, text: &str, style: Style) {
        let text_width = UnicodeWidthStr::width(text).min(width as usize) as u16;
        let start = x + width.saturating_sub(text_width) / 2;
        self.text(start, y, text, width, style);
    }

    fn fill(&mut self, x: u16, y: u16, width: u16, height: u16, style: Style) {
        for row in y..y.saturating_add(height).min(self.height) {
            for column in x..x.saturating_add(width).min(self.width) {
                self.put(column, row, ' ', style);
            }
        }
    }

    fn hline(&mut self, x: u16, y: u16, width: u16, ch: char, style: Style) {
        for column in x..x.saturating_add(width).min(self.width) {
            self.put(column, y, ch, style);
        }
    }

    fn border(&mut self, x: u16, y: u16, width: u16, height: u16, style: Style) {
        if width < 2 || height < 2 {
            return;
        }
        self.put(x, y, '+', style);
        self.put(x + width - 1, y, '+', style);
        self.put(x, y + height - 1, '+', style);
        self.put(x + width - 1, y + height - 1, '+', style);
        self.hline(x + 1, y, width - 2, '-', style);
        self.hline(x + 1, y + height - 1, width - 2, '-', style);
        for row in y + 1..y + height - 1 {
            self.put(x, row, '|', style);
            self.put(x + width - 1, row, '|', style);
        }
    }

    fn render(&self, output: &mut Stdout) -> io::Result<()> {
        output.queue(BeginSynchronizedUpdate)?;
        let mut active_style: Option<Style> = None;

        for y in 0..self.height {
            output.queue(MoveTo(0, y))?;
            let mut x = 0;
            while x < self.width {
                let index = y as usize * self.width as usize + x as usize;
                let style = self.cells[index].style;
                let mut span = String::new();

                while x < self.width {
                    let index = y as usize * self.width as usize + x as usize;
                    let cell = self.cells[index];
                    if cell.style != style {
                        break;
                    }
                    if cell.ch != '\0' {
                        span.push(cell.ch);
                    }
                    x += 1;
                }

                if active_style != Some(style) {
                    output.queue(SetForegroundColor(style.fg))?;
                    output.queue(SetBackgroundColor(style.bg))?;
                    output.queue(SetAttribute(if style.bold {
                        Attribute::Bold
                    } else {
                        Attribute::NormalIntensity
                    }))?;
                    active_style = Some(style);
                }
                output.queue(Print(span))?;
            }
        }

        output.queue(ResetColor)?;
        output.queue(SetAttribute(Attribute::Reset))?;
        output.queue(EndSynchronizedUpdate)?;
        output.flush()
    }
}

pub struct Terminal {
    output: Stdout,
}

impl Terminal {
    pub fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut output = io::stdout();
        if let Err(error) = execute!(
            output,
            EnterAlternateScreen,
            DisableLineWrap,
            Hide,
            Clear(ClearType::All),
            SetTitle("MUSA")
        ) {
            let _ = terminal::disable_raw_mode();
            return Err(error);
        }
        Ok(Self { output })
    }

    pub fn draw(&mut self, app: &App) -> io::Result<()> {
        let (width, height) = terminal::size()?;
        let palette = Palette::for_theme(app.theme);
        let mut canvas = Canvas::new(width, height, palette);

        if width < 72 || height < 24 {
            draw_too_small(&mut canvas, palette);
        } else {
            draw_app(&mut canvas, app, palette);
        }

        canvas.render(&mut self.output)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = execute!(
            self.output,
            ResetColor,
            SetAttribute(Attribute::Reset),
            EndSynchronizedUpdate,
            Show,
            EnableLineWrap,
            LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}

fn draw_app(canvas: &mut Canvas, app: &App, palette: Palette) {
    let player_height = 6;
    let upper_height = canvas.height.saturating_sub(player_height);
    let sidebar_width = if canvas.width >= 110 { 24 } else { 20 };
    let content_x = sidebar_width;
    let content_width = canvas.width.saturating_sub(sidebar_width);
    let top_height = 4;

    draw_sidebar(canvas, app, palette, sidebar_width, upper_height);
    draw_topbar(
        canvas,
        app,
        palette,
        content_x,
        content_width,
        top_height,
    );
    draw_content(
        canvas,
        app,
        palette,
        content_x,
        top_height,
        content_width,
        upper_height.saturating_sub(top_height),
    );
    draw_player(canvas, app, palette, upper_height, player_height);

    if app.help_open {
        draw_help(canvas, palette);
    } else if app.text_input.is_some() {
        draw_text_input(canvas, app, palette);
    } else if app.playlist_picker.is_some() {
        draw_playlist_picker(canvas, app, palette);
    }
}

fn draw_sidebar(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    width: u16,
    height: u16,
) {
    canvas.fill(0, 0, width, height, Style::new(palette.text, palette.sidebar));
    canvas.text(2, 1, "MUSA", width.saturating_sub(4), Style::new(palette.accent, palette.sidebar).bold());

    for (index, view) in View::ALL.iter().copied().enumerate() {
        let row = 4 + index as u16 * 2;
        let active = app.view == view;
        let background = if active { palette.selected } else { palette.sidebar };
        canvas.fill(0, row, width, 1, Style::new(palette.text, background));
        if active {
            canvas.fill(0, row, 1, 1, Style::new(palette.accent, palette.accent));
        }
        let label = view.label();
        canvas.text(
            2,
            row,
            label,
            width.saturating_sub(3),
            if active {
                Style::new(palette.text, background).bold()
            } else {
                Style::new(palette.muted, background)
            },
        );
    }

    if height >= 28 {
        let info_y = height.saturating_sub(7);
        canvas.text(2, info_y, "YOUR LIBRARY", width.saturating_sub(4), Style::new(palette.faint, palette.sidebar).bold());
        canvas.text(
            2,
            info_y + 2,
            &format!("{} songs", app.tracks.len()),
            width.saturating_sub(4),
            Style::new(palette.muted, palette.sidebar),
        );
        canvas.text(
            2,
            info_y + 3,
            &format!("{} albums", app.albums.len()),
            width.saturating_sub(4),
            Style::new(palette.muted, palette.sidebar),
        );
        canvas.text(
            2,
            info_y + 4,
            &format!("{} artists", app.artists.len()),
            width.saturating_sub(4),
            Style::new(palette.muted, palette.sidebar),
        );
        canvas.text(
            2,
            info_y + 5,
            &format!("{} playlists", app.playlists.len()),
            width.saturating_sub(4),
            Style::new(palette.muted, palette.sidebar),
        );
    }
}

fn draw_topbar(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    width: u16,
    height: u16,
) {
    canvas.fill(x, 0, width, height, Style::new(palette.text, palette.surface));

    if app.view == View::Search {
        let box_width = width.saturating_sub(24).clamp(24, 60);
        canvas.fill(x + 2, 1, box_width, 2, Style::new(palette.text, palette.surface_alt));
        let cursor = if app.search_editing { "_" } else { "" };
        let query = if app.search_query.is_empty() {
            if app.search_editing {
                format!("Search library{cursor}")
            } else {
                "Search".to_owned()
            }
        } else {
            format!("{}{cursor}", app.search_query)
        };
        canvas.text(
            x + 4,
            2,
            &query,
            box_width.saturating_sub(4),
            Style::new(
                if app.search_query.is_empty() { palette.muted } else { palette.text },
                palette.surface_alt,
            ),
        );
    } else {
        canvas.text(
            x + 3,
            1,
            app.view.label(),
            width.saturating_sub(6),
            Style::new(palette.text, palette.surface).bold(),
        );
    }

    let scan = match app.scan_phase {
        ScanPhase::Idle => None,
        ScanPhase::Discovering => Some("Scanning folders...".to_owned()),
        ScanPhase::Reading { done, total } => Some(format!("Tags {done}/{total}")),
    };
    if let Some(scan) = scan {
        canvas.text_right(
            x + width.saturating_sub(2),
            1,
            &scan,
            24,
            Style::new(palette.muted, palette.surface),
        );
    }
}

fn draw_content(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    canvas.fill(x, y, width, height, Style::new(palette.text, palette.background));
    match app.view {
        View::Home => draw_home(canvas, app, palette, x, y, width, height),
        View::Search => draw_search(canvas, app, palette, x, y, width, height),
        View::Songs => draw_songs(canvas, app, palette, x, y, width, height),
        View::Albums => draw_albums(canvas, app, palette, x, y, width, height),
        View::Artists => draw_artists(canvas, app, palette, x, y, width, height),
        View::Playlists => draw_playlists(canvas, app, palette, x, y, width, height),
        View::Folders => draw_folders(canvas, app, palette, x, y, width, height),
    }
}

fn draw_home(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    let card_gap = 2;
    let card_width = width.saturating_sub(8 + card_gap * 3) / 4;
    let card_y = y + 1;
    let labels = [
        (app.tracks.len().to_string(), "Songs"),
        (app.albums.len().to_string(), "Albums"),
        (app.artists.len().to_string(), "Artists"),
        (app.playlists.len().to_string(), "Playlists"),
    ];
    for (index, (value, label)) in labels.iter().enumerate() {
        let card_x = x + 3 + index as u16 * (card_width + card_gap);
        canvas.fill(card_x, card_y, card_width, 4, Style::new(palette.text, palette.surface_alt));
        canvas.text(card_x + 2, card_y + 1, value, card_width.saturating_sub(4), Style::new(palette.text, palette.surface_alt).bold());
        canvas.text(card_x + 2, card_y + 2, label, card_width.saturating_sub(4), Style::new(palette.muted, palette.surface_alt));
    }

    let list_y = y + 7;
    canvas.text(x + 3, list_y, "Recently played", width.saturating_sub(6), Style::new(palette.text, palette.background).bold());
    let recent = app.recent_indices();
    if recent.is_empty() {
        empty_message(
            canvas,
            x,
            list_y + 3,
            width,
            "Your recently played songs will appear here",
            palette,
        );
    } else {
        draw_track_table(
            canvas,
            app,
            palette,
            &recent,
            app.selected,
            x + 2,
            list_y + 2,
            width.saturating_sub(4),
            height.saturating_sub(10),
            TrackColumns::Album,
        );
    }
}

fn draw_songs(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    let ids: Vec<usize> = (0..app.tracks.len()).collect();
    if ids.is_empty() {
        empty_library(canvas, x, y, width, palette);
        return;
    }
    draw_track_table(
        canvas,
        app,
        palette,
        &ids,
        app.selected,
        x + 2,
        y + 1,
        width.saturating_sub(4),
        height.saturating_sub(2),
        TrackColumns::Album,
    );
}

fn draw_search(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    if app.search_query.trim().is_empty() {
        return;
    }
    if app.search_results.is_empty() {
        empty_message(canvas, x, y + height / 3, width, "No matches", palette);
        return;
    }

    let inner_x = x + 2;
    let inner_width = width.saturating_sub(4);
    canvas.text(inner_x + 2, y + 1, "#", 4, Style::new(palette.faint, palette.background));
    canvas.text(inner_x + 7, y + 1, "TYPE", 9, Style::new(palette.faint, palette.background));
    canvas.text(inner_x + 18, y + 1, "RESULT", inner_width.saturating_sub(20), Style::new(palette.faint, palette.background));
    canvas.hline(inner_x, y + 2, inner_width, '-', Style::new(palette.border, palette.background));

    let visible = height.saturating_sub(4) as usize;
    let start = window_start(app.selected, app.search_results.len(), visible);
    for (row, position) in (start..app.search_results.len()).take(visible).enumerate() {
        let item = app.search_results[position];
        let row_y = y + 3 + row as u16;
        let selected = position == app.selected;
        let (kind, primary, secondary, current) = match item {
            SearchItem::Playlist(index) => {
                let playlist = &app.playlists[index];
                (
                    "PLAYLIST",
                    playlist.name.as_str(),
                    format!("{} available songs", playlist.tracks.len()),
                    false,
                )
            }
            SearchItem::Artist(index) => {
                let artist = &app.artists[index];
                (
                    "ARTIST",
                    artist.name.as_str(),
                    format!("{} albums, {} songs", artist.album_count, artist.tracks.len()),
                    false,
                )
            }
            SearchItem::Album(index) => {
                let album = &app.albums[index];
                (
                    "ALBUM",
                    album.title.as_str(),
                    format!("{} - {} songs", album.artist, album.tracks.len()),
                    false,
                )
            }
            SearchItem::Track(index) => {
                let track = &app.tracks[index];
                (
                    "SONG",
                    track.title.as_str(),
                    format!("{} - {}", track.artist, track.album),
                    app.current == Some(index),
                )
            }
        };
        let background = row_background(selected, current, palette);
        canvas.fill(inner_x, row_y, inner_width, 1, Style::new(palette.text, background));
        canvas.text(inner_x + 1, row_y, if current { ">" } else { " " }, 1, Style::new(palette.accent, background).bold());
        canvas.text(inner_x + 3, row_y, &format!("{:>4}", position + 1), 4, Style::new(palette.muted, background));
        canvas.text(inner_x + 8, row_y, kind, 8, Style::new(if kind == "SONG" { palette.accent } else { palette.muted }, background).bold());
        let primary_width = inner_width.saturating_mul(45) / 100;
        canvas.text(inner_x + 18, row_y, primary, primary_width.saturating_sub(18), selected_style(selected, background, palette));
        canvas.text(inner_x + primary_width, row_y, &secondary, inner_width.saturating_sub(primary_width + 1), Style::new(palette.muted, background));
    }
}

fn draw_albums(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    if let Some(DetailView::Album(index)) = app.detail {
        if let Some(album) = app.albums.get(index) {
            draw_collection_header(
                canvas,
                palette,
                x,
                y,
                width,
                "ALBUM",
                &album.title,
                &format!("{}  -  {} songs  -  {}", album.artist, album.tracks.len(), format_duration(album.duration)),
            );
            draw_track_table(
                canvas,
                app,
                palette,
                &album.tracks,
                app.selected,
                x + 2,
                y + 5,
                width.saturating_sub(4),
                height.saturating_sub(6),
                TrackColumns::TrackNumber,
            );
        }
        return;
    }

    if app.albums.is_empty() {
        empty_library(canvas, x, y, width, palette);
        return;
    }

    let inner_x = x + 2;
    let inner_width = width.saturating_sub(4);
    canvas.text(inner_x + 3, y + 1, "#", 4, Style::new(palette.faint, palette.background));
    canvas.text(inner_x + 9, y + 1, "ALBUM", inner_width / 2, Style::new(palette.faint, palette.background));
    canvas.text(inner_x + inner_width / 2, y + 1, "ARTIST", inner_width / 3, Style::new(palette.faint, palette.background));
    canvas.text_right(inner_x + inner_width - 1, y + 1, "SONGS", 8, Style::new(palette.faint, palette.background));
    canvas.hline(inner_x, y + 2, inner_width, '-', Style::new(palette.border, palette.background));

    let visible = height.saturating_sub(4) as usize;
    let start = window_start(app.selected, app.albums.len(), visible);
    for (row, position) in (start..app.albums.len()).take(visible).enumerate() {
        let album = &app.albums[position];
        let row_y = y + 3 + row as u16;
        let selected = position == app.selected;
        let background = row_background(selected, false, palette);
        canvas.fill(inner_x, row_y, inner_width, 1, Style::new(palette.text, background));
        canvas.text(inner_x + 3, row_y, &format!("{:>4}", position + 1), 4, Style::new(palette.muted, background));
        canvas.text(inner_x + 9, row_y, &album.title, inner_width / 2 - 10, selected_style(selected, background, palette));
        canvas.text(inner_x + inner_width / 2, row_y, &album.artist, inner_width / 3, Style::new(palette.muted, background));
        canvas.text_right(inner_x + inner_width - 1, row_y, &album.tracks.len().to_string(), 8, Style::new(palette.muted, background));
    }
}

fn draw_artists(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    if let Some(DetailView::Artist(index)) = app.detail {
        if let Some(artist) = app.artists.get(index) {
            draw_collection_header(
                canvas,
                palette,
                x,
                y,
                width,
                "ARTIST",
                &artist.name,
                &format!("{} albums  -  {} songs", artist.album_count, artist.tracks.len()),
            );
            draw_track_table(
                canvas,
                app,
                palette,
                &artist.tracks,
                app.selected,
                x + 2,
                y + 5,
                width.saturating_sub(4),
                height.saturating_sub(6),
                TrackColumns::Album,
            );
        }
        return;
    }

    if app.artists.is_empty() {
        empty_library(canvas, x, y, width, palette);
        return;
    }

    let inner_x = x + 2;
    let inner_width = width.saturating_sub(4);
    canvas.text(inner_x + 3, y + 1, "#", 4, Style::new(palette.faint, palette.background));
    canvas.text(inner_x + 9, y + 1, "ARTIST", inner_width / 2, Style::new(palette.faint, palette.background));
    canvas.text_right(inner_x + inner_width - 12, y + 1, "ALBUMS", 8, Style::new(palette.faint, palette.background));
    canvas.text_right(inner_x + inner_width - 1, y + 1, "SONGS", 8, Style::new(palette.faint, palette.background));
    canvas.hline(inner_x, y + 2, inner_width, '-', Style::new(palette.border, palette.background));

    let visible = height.saturating_sub(4) as usize;
    let start = window_start(app.selected, app.artists.len(), visible);
    for (row, position) in (start..app.artists.len()).take(visible).enumerate() {
        let artist = &app.artists[position];
        let row_y = y + 3 + row as u16;
        let selected = position == app.selected;
        let background = row_background(selected, false, palette);
        canvas.fill(inner_x, row_y, inner_width, 1, Style::new(palette.text, background));
        canvas.text(inner_x + 3, row_y, &format!("{:>4}", position + 1), 4, Style::new(palette.muted, background));
        canvas.text(inner_x + 9, row_y, &artist.name, inner_width.saturating_sub(34), selected_style(selected, background, palette));
        canvas.text_right(inner_x + inner_width - 12, row_y, &artist.album_count.to_string(), 8, Style::new(palette.muted, background));
        canvas.text_right(inner_x + inner_width - 1, row_y, &artist.tracks.len().to_string(), 8, Style::new(palette.muted, background));
    }
}

fn draw_playlists(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    if let Some(DetailView::Playlist(index)) = app.detail {
        if let Some(playlist) = app.playlists.get(index) {
            let missing = playlist.track_paths.len().saturating_sub(playlist.tracks.len());
            let subtitle = if missing == 0 {
                format!(
                    "{} songs  -  {}",
                    playlist.tracks.len(),
                    format_duration(playlist.duration)
                )
            } else {
                format!(
                    "{} available songs  -  {} unavailable  -  {}",
                    playlist.tracks.len(),
                    missing,
                    format_duration(playlist.duration)
                )
            };
            draw_collection_header(
                canvas,
                palette,
                x,
                y,
                width,
                "PLAYLIST",
                &playlist.name,
                &subtitle,
            );
            if playlist.tracks.is_empty() {
                empty_message(
                    canvas,
                    x,
                    y + 7,
                    width,
                    if playlist.track_paths.is_empty() {
                        "This playlist is empty"
                    } else {
                        "Playlist songs are currently unavailable"
                    },
                    palette,
                );
            } else {
                draw_track_table(
                    canvas,
                    app,
                    palette,
                    &playlist.tracks,
                    app.selected,
                    x + 2,
                    y + 5,
                    width.saturating_sub(4),
                    height.saturating_sub(6),
                    TrackColumns::Album,
                );
            }
        }
        return;
    }

    if app.playlists.is_empty() {
        empty_message(canvas, x, y + height / 3, width, "No custom playlists", palette);
        return;
    }

    let inner_x = x + 2;
    let inner_width = width.saturating_sub(4);
    canvas.text(inner_x + 3, y + 1, "#", 4, Style::new(palette.faint, palette.background));
    canvas.text(inner_x + 9, y + 1, "PLAYLIST", inner_width / 2, Style::new(palette.faint, palette.background));
    canvas.text_right(inner_x + inner_width - 12, y + 1, "SONGS", 8, Style::new(palette.faint, palette.background));
    canvas.text_right(inner_x + inner_width - 1, y + 1, "TIME", 8, Style::new(palette.faint, palette.background));
    canvas.hline(inner_x, y + 2, inner_width, '-', Style::new(palette.border, palette.background));

    let visible = height.saturating_sub(4) as usize;
    let start = window_start(app.selected, app.playlists.len(), visible);
    for (row, position) in (start..app.playlists.len()).take(visible).enumerate() {
        let playlist = &app.playlists[position];
        let row_y = y + 3 + row as u16;
        let selected = position == app.selected;
        let background = row_background(selected, false, palette);
        canvas.fill(inner_x, row_y, inner_width, 1, Style::new(palette.text, background));
        canvas.text(inner_x + 3, row_y, &format!("{:>4}", position + 1), 4, Style::new(palette.muted, background));
        canvas.text(inner_x + 9, row_y, &playlist.name, inner_width.saturating_sub(32), selected_style(selected, background, palette));
        let songs = if playlist.tracks.len() == playlist.track_paths.len() {
            playlist.tracks.len().to_string()
        } else {
            format!("{}/{}", playlist.tracks.len(), playlist.track_paths.len())
        };
        canvas.text_right(inner_x + inner_width - 12, row_y, &songs, 8, Style::new(palette.muted, background));
        canvas.text_right(inner_x + inner_width - 1, row_y, &format_duration(playlist.duration), 8, Style::new(palette.muted, background));
    }
}

fn draw_folders(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
) {
    let gap = 1;
    let roots_width = (width * 38 / 100).max(24);
    let browser_x = x + roots_width + gap;
    let browser_width = width.saturating_sub(roots_width + gap);

    canvas.fill(x, y, roots_width, height, Style::new(palette.text, palette.surface));
    canvas.fill(browser_x, y, browser_width, height, Style::new(palette.text, palette.background));

    let roots_active = app.folder_focus == FolderFocus::Roots;
    let browser_active = app.folder_focus == FolderFocus::Browser;
    canvas.text(
        x + 2,
        y + 1,
        "LIBRARY FOLDERS",
        roots_width.saturating_sub(4),
        Style::new(if roots_active { palette.accent } else { palette.text }, palette.surface).bold(),
    );
    canvas.text(
        browser_x + 2,
        y + 1,
        "FOLDER BROWSER",
        browser_width.saturating_sub(4),
        Style::new(if browser_active { palette.accent } else { palette.text }, palette.background).bold(),
    );
    canvas.text(
        browser_x + 2,
        y + 2,
        &app.browser_dir.to_string_lossy(),
        browser_width.saturating_sub(4),
        Style::new(palette.muted, palette.background),
    );

    let list_y = y + 4;
    let list_height = height.saturating_sub(4) as usize;
    if app.roots.is_empty() {
        canvas.text(x + 2, list_y, "No folders added", roots_width.saturating_sub(4), Style::new(palette.muted, palette.surface));
    } else {
        let start = window_start(app.root_selected, app.roots.len(), list_height);
        for (row, position) in (start..app.roots.len()).take(list_height).enumerate() {
            let row_y = list_y + row as u16;
            let selected = roots_active && position == app.root_selected;
            let background = if selected { palette.selected } else { palette.surface };
            canvas.fill(x + 1, row_y, roots_width.saturating_sub(2), 1, Style::new(palette.text, background));
            canvas.text(x + 2, row_y, &format!("{:>2}", position + 1), 2, Style::new(palette.muted, background));
            canvas.text(x + 5, row_y, &app.roots[position].to_string_lossy(), roots_width.saturating_sub(7), selected_style(selected, background, palette));
        }
    }

    if app.browser_entries.is_empty() {
        canvas.text(browser_x + 2, list_y, "Folder is empty", browser_width.saturating_sub(4), Style::new(palette.muted, palette.background));
    } else {
        let start = window_start(app.browser_selected, app.browser_entries.len(), list_height);
        for (row, position) in (start..app.browser_entries.len()).take(list_height).enumerate() {
            let row_y = list_y + row as u16;
            let selected = browser_active && position == app.browser_selected;
            let background = if selected { palette.selected } else { palette.background };
            canvas.fill(browser_x + 1, row_y, browser_width.saturating_sub(2), 1, Style::new(palette.text, background));
            let entry = &app.browser_entries[position];
            match &entry.kind {
                DirectoryEntryKind::Directory => {
                    canvas.text(browser_x + 2, row_y, "DIR", 3, Style::new(palette.accent, background).bold());
                    canvas.text(
                        browser_x + 6,
                        row_y,
                        &entry.name,
                        browser_width.saturating_sub(8),
                        selected_style(selected, background, palette),
                    );
                }
                DirectoryEntryKind::Track(track) => {
                    let is_current = app
                        .current_track()
                        .is_some_and(|current| current.path.as_path() == track.path.as_path());
                    canvas.text(
                        browser_x + 2,
                        row_y,
                        if is_current { ">" } else { "♪" },
                        1,
                        Style::new(palette.accent, background).bold(),
                    );

                    let duration_width = 7;
                    if browser_width >= 56 {
                        let number = match (track.disc_no, track.track_no) {
                            (Some(disc), Some(track)) => format!("{disc}.{track:02}"),
                            (None, Some(track)) => format!("{track:02}"),
                            _ => String::new(),
                        };
                        if !number.is_empty() {
                            canvas.text(browser_x + 4, row_y, &number, 5, Style::new(palette.faint, background));
                        }

                        let title_x = browser_x + 10;
                        let artist_width = browser_width.saturating_mul(30) / 100;
                        let artist_x = browser_x
                            + browser_width
                                .saturating_sub(artist_width + duration_width + 2);
                        let title_width = artist_x.saturating_sub(title_x + 1);
                        canvas.text(
                            title_x,
                            row_y,
                            &track.title,
                            title_width,
                            selected_style(selected, background, palette),
                        );
                        canvas.text(
                            artist_x,
                            row_y,
                            &track.artist,
                            artist_width,
                            Style::new(palette.muted, background),
                        );
                    } else {
                        canvas.text(
                            browser_x + 4,
                            row_y,
                            &track.title,
                            browser_width.saturating_sub(duration_width + 7),
                            selected_style(selected, background, palette),
                        );
                    }
                    canvas.text_right(
                        browser_x + browser_width.saturating_sub(2),
                        row_y,
                        &track
                            .duration
                            .map(format_duration)
                            .unwrap_or_else(|| "--:--".to_owned()),
                        duration_width,
                        Style::new(palette.muted, background),
                    );
                }
            }
        }
    }

}

#[derive(Clone, Copy)]
enum TrackColumns {
    Album,
    TrackNumber,
}

#[allow(clippy::too_many_arguments)]
fn draw_track_table(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    ids: &[usize],
    selected: usize,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    columns: TrackColumns,
) {
    if width < 30 || height < 3 {
        return;
    }

    let title_x = x + 9;
    let middle_x = x + width * 58 / 100;
    let time_width = 8;
    let time_x = x + width.saturating_sub(time_width + 1);

    canvas.text(x + 2, y, "#", 5, Style::new(palette.faint, palette.background));
    canvas.text(title_x, y, "TITLE", middle_x.saturating_sub(title_x + 1), Style::new(palette.faint, palette.background));
    canvas.text(
        middle_x,
        y,
        match columns {
            TrackColumns::Album => "ALBUM",
            TrackColumns::TrackNumber => "ARTIST",
        },
        time_x.saturating_sub(middle_x + 1),
        Style::new(palette.faint, palette.background),
    );
    canvas.text_right(x + width - 1, y, "TIME", time_width, Style::new(palette.faint, palette.background));
    canvas.hline(x, y + 1, width, '-', Style::new(palette.border, palette.background));

    let visible = height.saturating_sub(2) as usize;
    let start = window_start(selected, ids.len(), visible);
    for (row, position) in (start..ids.len()).take(visible).enumerate() {
        let track_index = ids[position];
        let track = &app.tracks[track_index];
        let row_y = y + 2 + row as u16;
        let is_selected = position == selected;
        let is_current = app.current == Some(track_index);
        let background = row_background(is_selected, is_current, palette);
        canvas.fill(x, row_y, width, 1, Style::new(palette.text, background));

        let marker = if is_current {
            match app.state {
                PlaybackState::Playing => ">",
                PlaybackState::Paused => "=",
                PlaybackState::Stopped => "*",
            }
        } else {
            " "
        };
        canvas.text(x + 1, row_y, marker, 1, Style::new(palette.accent, background).bold());
        canvas.text(x + 3, row_y, &format!("{:>5}", position + 1), 5, Style::new(palette.muted, background));

        let title = match columns {
            TrackColumns::Album => track.title.clone(),
            TrackColumns::TrackNumber => format!("[{}] {}", track_number(track), track.title),
        };
        canvas.text(
            title_x,
            row_y,
            &title,
            middle_x.saturating_sub(title_x + 1),
            selected_style(is_selected, background, palette),
        );
        let middle = match columns {
            TrackColumns::Album => track.album.as_str(),
            TrackColumns::TrackNumber => track.artist.as_str(),
        };
        canvas.text(
            middle_x,
            row_y,
            middle,
            time_x.saturating_sub(middle_x + 1),
            Style::new(palette.muted, background),
        );
        canvas.text_right(
            x + width - 1,
            row_y,
            &track.duration.map(format_duration).unwrap_or_else(|| "--:--".to_owned()),
            time_width,
            Style::new(palette.muted, background),
        );
    }
}

fn draw_collection_header(
    canvas: &mut Canvas,
    palette: Palette,
    x: u16,
    y: u16,
    width: u16,
    kind: &str,
    title: &str,
    subtitle: &str,
) {
    canvas.fill(x, y, width, 4, Style::new(palette.text, palette.surface_alt));
    canvas.text(x + 3, y, kind, width.saturating_sub(6), Style::new(palette.accent, palette.surface_alt).bold());
    canvas.text(x + 3, y + 1, title, width.saturating_sub(6), Style::new(palette.text, palette.surface_alt).bold());
    canvas.text(x + 3, y + 2, subtitle, width.saturating_sub(6), Style::new(palette.muted, palette.surface_alt));
}

fn draw_player(
    canvas: &mut Canvas,
    app: &App,
    palette: Palette,
    y: u16,
    height: u16,
) {
    canvas.fill(0, y, canvas.width, height, Style::new(palette.text, palette.player));
    canvas.hline(0, y, canvas.width, '-', Style::new(palette.border, palette.player));

    let left_width = (canvas.width * 30 / 100).max(24);
    let right_width = 22;
    let center_x = left_width;
    let center_width = canvas.width.saturating_sub(left_width + right_width);

    if let Some(track) = app.current_track() {
        canvas.text(2, y + 1, &track.title, left_width.saturating_sub(4), Style::new(palette.text, palette.player).bold());
        canvas.text(2, y + 2, &format!("{} - {}", track.artist, track.album), left_width.saturating_sub(4), Style::new(palette.muted, palette.player));
    } else {
        canvas.text(2, y + 1, "Nothing playing", left_width.saturating_sub(4), Style::new(palette.muted, palette.player));
    }

    let play_label = match app.state {
        PlaybackState::Playing => "PAUSE",
        PlaybackState::Paused | PlaybackState::Stopped => "PLAY",
    };
    let controls = format!(
        "{}   PREV   {}   NEXT   REPEAT:{}",
        if app.shuffle { "SHUFFLE" } else { "shuffle" },
        play_label,
        app.repeat.label()
    );
    canvas.text_center(center_x, y + 1, center_width, &controls, Style::new(palette.text, palette.player).bold());

    let position = app.position_seconds();
    let total = app.total_seconds();
    let left_time = format_time(position);
    let right_time = total.map(format_time).unwrap_or_else(|| "--:--".to_owned());
    let progress_x = center_x + 8;
    let progress_width = center_width.saturating_sub(18);
    canvas.text(center_x + 1, y + 3, &left_time, 6, Style::new(palette.muted, palette.player));
    canvas.text_right(center_x + center_width.saturating_sub(1), y + 3, &right_time, 6, Style::new(palette.muted, palette.player));
    if progress_width > 0 {
        let ratio = total
            .filter(|value| *value > 0.0)
            .map(|value| (position / value).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let filled = (progress_width as f64 * ratio).round() as usize;
        let bar = format!("{}{}", "=".repeat(filled), "-".repeat(progress_width as usize - filled));
        canvas.text(progress_x, y + 3, &bar, progress_width, Style::new(palette.accent, palette.player));
    }

    canvas.text_right(
        canvas.width.saturating_sub(2),
        y + 1,
        &format!("VOL {:>3}%", app.volume),
        right_width.saturating_sub(2),
        Style::new(palette.text, palette.player).bold(),
    );
    let volume_bar_width = right_width.saturating_sub(8);
    let volume_filled = volume_bar_width as usize * app.volume as usize / 100;
    let volume_bar = format!("{}{}", "=".repeat(volume_filled), "-".repeat(volume_bar_width as usize - volume_filled));
    canvas.text_right(
        canvas.width.saturating_sub(2),
        y + 2,
        &volume_bar,
        volume_bar_width,
        Style::new(palette.accent, palette.player),
    );

    canvas.text(2, y + 4, &app.status, canvas.width.saturating_sub(4), Style::new(palette.muted, palette.player));
}

fn draw_text_input(canvas: &mut Canvas, app: &App, palette: Palette) {
    let Some(input) = app.text_input.as_ref() else {
        return;
    };
    let width = canvas.width.min(64).saturating_sub(4).max(28);
    let height = 7;
    let x = (canvas.width - width) / 2;
    let y = (canvas.height - height) / 2;

    canvas.fill(x, y, width, height, Style::new(palette.text, palette.surface));
    canvas.border(x, y, width, height, Style::new(palette.accent, palette.surface));
    canvas.text(x + 3, y + 1, &input.prompt, width.saturating_sub(6), Style::new(palette.text, palette.surface).bold());
    canvas.fill(x + 3, y + 3, width.saturating_sub(6), 1, Style::new(palette.text, palette.surface_alt));
    let value = format!("{}_", input.value);
    canvas.text(x + 4, y + 3, &value, width.saturating_sub(8), Style::new(palette.text, palette.surface_alt));
}

fn draw_playlist_picker(canvas: &mut Canvas, app: &App, palette: Palette) {
    let Some(picker) = app.playlist_picker.as_ref() else {
        return;
    };
    let width = canvas.width.min(68).saturating_sub(4).max(32);
    let max_rows = canvas.height.saturating_sub(10) as usize;
    let list_rows = app.playlists.len().min(max_rows).max(1);
    let height = (list_rows as u16 + 6).min(canvas.height.saturating_sub(4)).max(9);
    let x = (canvas.width - width) / 2;
    let y = (canvas.height - height) / 2;

    canvas.fill(x, y, width, height, Style::new(palette.text, palette.surface));
    canvas.border(x, y, width, height, Style::new(palette.accent, palette.surface));
    canvas.text(x + 3, y + 1, "Add to playlist", width.saturating_sub(6), Style::new(palette.text, palette.surface).bold());
    let subtitle = format!("{}  -  {} songs", picker.source_label, picker.track_paths.len());
    canvas.text(x + 3, y + 2, &subtitle, width.saturating_sub(6), Style::new(palette.muted, palette.surface));
    canvas.hline(x + 2, y + 3, width.saturating_sub(4), '-', Style::new(palette.border, palette.surface));

    let rows = height.saturating_sub(5) as usize;
    let start = window_start(picker.selected, app.playlists.len(), rows);
    for (row, position) in (start..app.playlists.len()).take(rows).enumerate() {
        let playlist = &app.playlists[position];
        let row_y = y + 4 + row as u16;
        let selected = position == picker.selected;
        let background = if selected { palette.selected } else { palette.surface };
        canvas.fill(x + 2, row_y, width.saturating_sub(4), 1, Style::new(palette.text, background));
        canvas.text(x + 3, row_y, if selected { ">" } else { " " }, 1, Style::new(palette.accent, background).bold());
        canvas.text(x + 5, row_y, &playlist.name, width.saturating_sub(18), selected_style(selected, background, palette));
        canvas.text_right(x + width.saturating_sub(3), row_y, &playlist.tracks.len().to_string(), 8, Style::new(palette.muted, background));
    }
}

fn draw_help(canvas: &mut Canvas, palette: Palette) {
    let width = canvas.width.min(82).saturating_sub(4);
    let height = canvas.height.min(24).saturating_sub(2);
    let x = (canvas.width - width) / 2;
    let y = (canvas.height - height) / 2;

    canvas.fill(x, y, width, height, Style::new(palette.text, palette.surface));
    canvas.border(x, y, width, height, Style::new(palette.accent, palette.surface));
    canvas.text(x + 3, y + 1, "Keyboard", width.saturating_sub(6), Style::new(palette.text, palette.surface).bold());

    let controls = [
        ("1..7", "Switch Home through Folders"),
        ("/", "Search; in Folders, open filesystem root /"),
        ("Up/Down, j/k", "Move; PgUp/PgDn ten rows; g/G first/last"),
        ("Enter / Esc", "Open or play / close detail or modal"),
        ("Space, n, p", "Play-pause / next / previous"),
        ("[ ], + -", "Seek by five seconds / change volume"),
        ("x / r / t", "Shuffle / repeat mode / dark-light theme"),
        ("a", "Add song, album, artist, or playlist"),
        ("Playlists c/e", "Create / rename playlist"),
        ("Playlists d/D", "Remove selected song / delete playlist"),
        ("Picker c/Enter", "Create target playlist / add to selected playlist"),
        ("Folders Left/Right", "Switch roots and directory browser"),
        ("Folders Enter/Backspace", "Open directory / go to parent"),
        ("Folders ~ / a / d", "Home / add folder / remove root"),
        ("u", "Rescan saved library folders"),
        ("? / q", "Close help / quit"),
    ];

    for (index, (key, action)) in controls.iter().take(height.saturating_sub(4) as usize).enumerate() {
        let row = y + 3 + index as u16;
        canvas.text(x + 3, row, key, 24, Style::new(palette.accent, palette.surface).bold());
        canvas.text(x + 28, row, action, width.saturating_sub(31), Style::new(palette.text, palette.surface));
    }
}

fn empty_library(canvas: &mut Canvas, x: u16, y: u16, width: u16, palette: Palette) {
    empty_message(
        canvas,
        x,
        y + 4,
        width,
        "Music library is empty.",
        palette,
    );
}

fn empty_message(
    canvas: &mut Canvas,
    x: u16,
    y: u16,
    width: u16,
    message: &str,
    palette: Palette,
) {
    canvas.text_center(x, y, width, message, Style::new(palette.muted, palette.background));
}

fn row_background(selected: bool, current: bool, palette: Palette) -> Color {
    if selected {
        palette.selected
    } else if current {
        palette.current
    } else {
        palette.background
    }
}

fn selected_style(selected: bool, background: Color, palette: Palette) -> Style {
    if selected {
        Style::new(palette.text, background).bold()
    } else {
        Style::new(palette.text, background)
    }
}

fn window_start(selected: usize, len: usize, visible: usize) -> usize {
    if len <= visible || visible == 0 {
        return 0;
    }
    selected
        .saturating_sub(visible / 2)
        .min(len.saturating_sub(visible))
}

fn track_number(track: &Track) -> String {
    match (track.disc_no, track.track_no) {
        (Some(disc), Some(number)) => format!("{disc}.{number:02}"),
        (None, Some(number)) => format!("{number:02}"),
        (Some(disc), None) => format!("{disc}.--"),
        (None, None) => "--".to_owned(),
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    format_time(duration.as_secs_f64())
}

fn format_time(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "--:--".to_owned();
    }
    let seconds = seconds.round() as u64;
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn draw_too_small(canvas: &mut Canvas, palette: Palette) {
    canvas.text(2, 2, "MUSA", 10, Style::new(palette.accent, palette.background).bold());
    canvas.text(2, 4, "Terminal is too small.", 40, Style::new(palette.text, palette.background).bold());
    canvas.text(2, 5, "Minimum size: 72 x 24", 40, Style::new(palette.muted, palette.background));
}
