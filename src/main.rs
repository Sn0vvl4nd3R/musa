mod app;
mod audio;
mod library;
mod storage;
mod ui;

use std::{io, time::{Duration, Instant}};

use app::App;
use crossterm::event::{self, Event, KeyEventKind};
use ui::Terminal;

type Result<T> = io::Result<T>;

fn main() -> Result<()> {
    let mut app = App::new()?;
    let mut terminal = Terminal::enter()?;
    let tick_rate = Duration::from_millis(100);

    loop {
        let frame_started = Instant::now();

        app.tick();
        terminal.draw(&app)?;

        let wait = tick_rate.saturating_sub(frame_started.elapsed());
        if event::poll(wait)? {
            let event = event::read()?;
            if let Event::Key(key) = event {
                if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                    && app.handle_key(key)
                {
                    break;
                }
            }
        }
    }

    Ok(())
}
