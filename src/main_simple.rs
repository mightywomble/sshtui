mod config;
mod dashboard;
mod ui_simple;

use anyhow::Result;
use config::Config;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use std::io;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Keys,
    Groups, 
    Hosts,
}

#[derive(Debug)]
struct AppState {
    config: Config,
    focus_area: FocusArea,
    selected_key: usize,
    selected_group: usize,
    selected_host: usize,
    message: String,
    terminal_size: (u16, u16),
}

impl AppState {
    fn new() -> Result<Self> {
        let config = Config::load()?;
        
        Ok(Self {
            config,
            focus_area: FocusArea::Keys,
            selected_key: 0,
            selected_group: 0,
            selected_host: 0,
            message: String::new(),
            terminal_size: (120, 40),
        })
    }

    fn set_message(&mut self, message: String) {
        self.message = message;
    }

    fn advance_focus(&mut self, forward: bool) {
        if forward {
            self.focus_area = match self.focus_area {
                FocusArea::Keys => FocusArea::Groups,
                FocusArea::Groups => FocusArea::Hosts, 
                FocusArea::Hosts => FocusArea::Keys,
            };
        } else {
            self.focus_area = match self.focus_area {
                FocusArea::Keys => FocusArea::Hosts,
                FocusArea::Groups => FocusArea::Keys,
                FocusArea::Hosts => FocusArea::Groups,
            };
        }
    }

    fn update_layout(&mut self, terminal_size: (u16, u16)) {
        self.terminal_size = terminal_size;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app state
    let mut app = AppState::new()?;
    
    // Main event loop
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16); // ~60 FPS
    
    loop {
        // Handle terminal events
        if event::poll(Duration::from_millis(1))? {
            match event::read()? {
                Event::Key(key) => {
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) | 
                        (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                            break;
                        },
                        (KeyCode::Tab, KeyModifiers::NONE) => {
                            app.advance_focus(true);
                        },
                        (KeyCode::BackTab, _) => {
                            app.advance_focus(false);
                        },
                        (KeyCode::Up, _) => {
                            match app.focus_area {
                                FocusArea::Keys => {
                                    if app.selected_key > 0 {
                                        app.selected_key -= 1;
                                    }
                                },
                                FocusArea::Groups => {
                                    if app.selected_group > 0 {
                                        app.selected_group -= 1;
                                    }
                                },
                                FocusArea::Hosts => {
                                    if app.selected_host > 0 {
                                        app.selected_host -= 1;
                                    }
                                },
                            }
                        },
                        (KeyCode::Down, _) => {
                            match app.focus_area {
                                FocusArea::Keys => {
                                    if app.selected_key < app.config.keys.len().saturating_sub(1) {
                                        app.selected_key += 1;
                                    }
                                },
                                FocusArea::Groups => {
                                    if app.selected_group < app.config.groups.len().saturating_sub(1) {
                                        app.selected_group += 1;
                                    }
                                },
                                FocusArea::Hosts => {
                                    let hosts = app.config.get_hosts_for_group(app.selected_group);
                                    if app.selected_host < hosts.len().saturating_sub(1) {
                                        app.selected_host += 1;
                                    }
                                },
                            }
                        },
                        (KeyCode::Enter, _) => {
                            if app.focus_area == FocusArea::Hosts {
                                let hosts = app.config.get_hosts_for_group(app.selected_group);
                                if let Some(host) = hosts.get(app.selected_host) {
                                    app.set_message(format!("Would connect to {}", host.name));
                                }
                            }
                        },
                        _ => {}
                    }
                },
                Event::Resize(width, height) => {
                    app.update_layout((width, height));
                },
                _ => {}
            }
        }
        
        // Render UI
        terminal.draw(|frame| {
            ui_simple::render(frame, &mut app);
        })?;
        
        // Control frame rate
        let now = Instant::now();
        if now.duration_since(last_tick) >= tick_rate {
            last_tick = now;
        }
    }
    
    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    
    Ok(())
}
