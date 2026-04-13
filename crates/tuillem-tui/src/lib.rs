pub mod app;
pub mod commands;
pub mod control;
pub mod conversation;
pub mod help;
pub mod input;
pub mod settings;
pub mod sidebar;
pub mod theme;

use std::time::Duration;

use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event as CrosstermEvent,
        KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;
use tuillem_core::actions::Event;

use tracing::debug;

use crate::app::App;

pub async fn run(
    mut app: App,
    mut event_rx: mpsc::UnboundedReceiver<Event>,
    mouse_enabled: bool,
) -> anyhow::Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();

    // Enable keyboard enhancement for Shift+Enter detection
    let keyboard_enhanced = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
    )
    .is_ok();

    if mouse_enabled {
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    } else {
        execute!(stdout, EnterAlternateScreen)?;
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        // Force full redraw if needed (e.g. after external editor)
        if app.needs_redraw {
            terminal.clear()?;
            app.needs_redraw = false;
        }

        // Draw
        terminal.draw(|frame| {
            app.draw(frame);
        })?;

        // Process backend events (non-blocking)
        let mut event_count = 0;
        while let Ok(event) = event_rx.try_recv() {
            debug!("TUI received event: {:?}", event);
            app.apply_event(&event);
            event_count += 1;
        }
        if event_count > 0 {
            debug!("TUI processed {} events this frame", event_count);
        }

        // Poll terminal events with 16ms timeout (~60fps)
        if event::poll(Duration::from_millis(16))? {
            match event::read()? {
                CrosstermEvent::Key(key) => {
                    app.handle_key_event(key);
                }
                CrosstermEvent::Mouse(mouse) => {
                    app.handle_mouse_event(mouse);
                }
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Cleanup
    if keyboard_enhanced {
        let _ = execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags);
    }
    disable_raw_mode()?;
    if mouse_enabled {
        execute!(
            terminal.backend_mut(),
            DisableMouseCapture,
            LeaveAlternateScreen
        )?;
    } else {
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    }
    terminal.show_cursor()?;

    Ok(())
}
