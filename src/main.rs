mod app;
mod audio;
mod library;
mod storage;
mod ui;

use std::io;

use app::App;
use crossterm::event::{self, Event, KeyEventKind};
use ui::Terminal;

type Result<T> = io::Result<T>;

fn main() -> Result<()> {
    let mut app = App::new();
    let mut terminal = Terminal::enter()?;

    let mut redraw = true;
    let mut progress_epoch = app.progress_epoch();

    loop {
        redraw |= app.tick();

        let next_progress_epoch = app.progress_epoch();
        if next_progress_epoch != progress_epoch {
            progress_epoch = next_progress_epoch;
            redraw = true;
        }

        if redraw {
            terminal.draw(&app)?;
            redraw = false;
        }

        if !event::poll(app.poll_interval())? {
            continue;
        }

        match event::read()? {
            Event::Key(key)
                if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) =>
            {
                if app.handle_key(key) {
                    break;
                }
                redraw = true;
            }
            Event::Resize(_, _) => redraw = true,
            _ => {}
        }
    }

    Ok(())
}
