use std::io::{stdout, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::{
  event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{backend::CrosstermBackend, Terminal};

use crate::app::App;
use crate::ui::draw;

pub(crate) fn run(tick_rate: u64, app: &mut App) -> Result<()> {
  let tick_rate = Duration::from_millis(tick_rate);

  enable_raw_mode()?;
  let mut stdout = stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // Setup input handling
  let (tx, rx) = mpsc::channel();
  enum Event<I> {
    Input(I),
    Tick,
  }
  thread::spawn(move || {
    let mut last_tick = Instant::now();
    loop {
      // poll for tick rate duration, if no events, sent tick event.
      if event::poll(tick_rate - last_tick.elapsed()).unwrap() {
        if let CEvent::Key(key) = event::read().unwrap() {
          tx.send(Event::Input(key)).unwrap();
        }
      }
      if last_tick.elapsed() >= tick_rate {
        tx.send(Event::Tick).unwrap();
        last_tick = Instant::now();
      }
    }
  });

  terminal.clear()?;
  loop {
    terminal.draw(|f| draw(f, app))?;
    match rx.recv()? {
      Event::Input(event) => match event.code {
        KeyCode::Char('q') => {
          disable_raw_mode()?;
          execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
          terminal.show_cursor()?;
          break;
        }
        // KeyCode::Char(c) => app.on_key(c),
        // KeyCode::Left => app.on_left(),
        // KeyCode::Up => app.on_up(),
        // KeyCode::Right => app.on_right(),
        // KeyCode::Down => app.on_down(),
        _ => {}
      },
      Event::Tick => {
        // app.on_tick();
      }
    }
    // if app.should_quit {
    //   break;
    // }
  }

  Ok(())
}
