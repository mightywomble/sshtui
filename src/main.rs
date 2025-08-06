mod config;
mod ssh;
mod terminal_panel;
mod ui;
mod dashboard;
mod modal;

use anyhow::Result;
use config::{Config, Host};
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers, MouseEvent, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use ssh::{SshClient, SshEvent};
use std::io;
use std::time::{Duration, Instant};
use terminal_panel::RawTerminalPanel;
use tokio::sync::mpsc;
use log::{debug, error, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    Keys,
    Groups,
    Hosts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusSubArea {
    Items,
    AddButton,
    EditButton,
    DeleteButton,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ModalState {
    None,
    AddKey(KeyEditForm),
    EditKey(usize, KeyEditForm),
    AddGroup(GroupEditForm),
    EditGroup(usize, GroupEditForm),
    AddHost(HostEditForm),
    EditHost(usize, HostEditForm),
    Confirm(String, ConfirmAction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KeyEditForm {
    name: String,
    path: String,
    is_default: bool,
    field_focus: usize, // 0=name, 1=path, 2=is_default
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupEditForm {
    name: String,
    color: String,
    field_focus: usize, // 0=name, 1=color
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HostEditForm {
    name: String,
    host: String,
    port: String,
    user: String,
    key_path: String,
    use_key_selector: bool, // If true, show key selector instead of path input
    selected_key_index: usize, // Index of selected key from config.keys
    field_focus: usize, // 0=name, 1=host, 2=port, 3=user, 4=key_selector_or_path
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfirmAction {
    DeleteKey(usize),
    DeleteGroup(usize),
    DeleteHost(usize),
}

struct AppState {
    config: Config,
    focus_area: FocusArea,
    focus_sub_area: FocusSubArea,
    selected_key: usize,
    selected_group: usize,
    selected_host: usize,
    ssh_client: SshClient,
    terminal_panel: RawTerminalPanel,
    ssh_event_receiver: Option<mpsc::UnboundedReceiver<SshEvent>>,
    message: String,
    message_type: MessageType,
    terminal_size: (u16, u16),
    modal_state: ModalState,
}

#[derive(Debug, Clone, Copy)]
enum MessageType {
    Info,
    Success,
    Error,
}

impl AppState {
    fn new() -> Result<Self> {
        let config = Config::load()?;
        
        // Initialize terminal panel with default size
        let terminal_bounds = Rect {
            x: 40,
            y: 2,
            width: 80,
            height: 20,
        };
        
        let terminal_panel = RawTerminalPanel::new(terminal_bounds);
        
        Ok(Self {
            config,
            focus_area: FocusArea::Keys,
            focus_sub_area: FocusSubArea::Items,
            selected_key: 0,
            selected_group: 0,
            selected_host: 0,
            ssh_client: SshClient::new(),
            terminal_panel,
            ssh_event_receiver: None,
            message: String::new(),
            message_type: MessageType::Info,
            terminal_size: (120, 40),
            modal_state: ModalState::None,
        })
    }

    fn set_message(&mut self, message: String, msg_type: MessageType) {
        self.message = message;
        self.message_type = msg_type;
    }

    fn clear_message(&mut self) {
        self.message.clear();
    }

    fn advance_focus(&mut self, forward: bool) {
        if forward {
            match self.focus_area {
                FocusArea::Keys => match self.focus_sub_area {
                    FocusSubArea::Items => self.focus_sub_area = FocusSubArea::AddButton,
                    FocusSubArea::AddButton => {
                        if !self.config.keys.is_empty() {
                            self.focus_sub_area = FocusSubArea::EditButton;
                        } else {
                            self.focus_area = FocusArea::Groups;
                            self.focus_sub_area = FocusSubArea::Items;
                        }
                    },
                    FocusSubArea::EditButton => {
                        if !self.config.keys.is_empty() {
                            self.focus_sub_area = FocusSubArea::DeleteButton;
                        } else {
                            self.focus_area = FocusArea::Groups;
                            self.focus_sub_area = FocusSubArea::Items;
                        }
                    },
                    FocusSubArea::DeleteButton => {
                        self.focus_area = FocusArea::Groups;
                        self.focus_sub_area = FocusSubArea::Items;
                    },
                },
                FocusArea::Groups => match self.focus_sub_area {
                    FocusSubArea::Items => self.focus_sub_area = FocusSubArea::AddButton,
                    FocusSubArea::AddButton => {
                        if self.config.groups.len() > 1 {
                            self.focus_sub_area = FocusSubArea::EditButton;
                        } else {
                            self.focus_area = FocusArea::Hosts;
                            self.focus_sub_area = FocusSubArea::Items;
                        }
                    },
                    FocusSubArea::EditButton => {
                        if self.config.groups.len() > 1 {
                            self.focus_sub_area = FocusSubArea::DeleteButton;
                        } else {
                            self.focus_area = FocusArea::Hosts;
                            self.focus_sub_area = FocusSubArea::Items;
                        }
                    },
                    FocusSubArea::DeleteButton => {
                        self.focus_area = FocusArea::Hosts;
                        self.focus_sub_area = FocusSubArea::Items;
                    },
                },
                FocusArea::Hosts => match self.focus_sub_area {
                    FocusSubArea::Items => self.focus_sub_area = FocusSubArea::AddButton,
                    FocusSubArea::AddButton => {
                        let hosts = self.config.get_hosts_for_group(self.selected_group);
                        if !hosts.is_empty() {
                            self.focus_sub_area = FocusSubArea::EditButton;
                        } else {
                            self.focus_area = FocusArea::Keys;
                            self.focus_sub_area = FocusSubArea::Items;
                        }
                    },
                    FocusSubArea::EditButton => {
                        let hosts = self.config.get_hosts_for_group(self.selected_group);
                        if !hosts.is_empty() {
                            self.focus_sub_area = FocusSubArea::DeleteButton;
                        } else {
                            self.focus_area = FocusArea::Keys;
                            self.focus_sub_area = FocusSubArea::Items;
                        }
                    },
                    FocusSubArea::DeleteButton => {
                        self.focus_area = FocusArea::Keys;
                        self.focus_sub_area = FocusSubArea::Items;
                    },
                },
            }
        } else {
            // Reverse direction logic (similar but backwards)
            match self.focus_area {
                FocusArea::Keys => {
                    self.focus_area = FocusArea::Hosts;
                    self.focus_sub_area = FocusSubArea::DeleteButton;
                },
                FocusArea::Groups => {
                    self.focus_area = FocusArea::Keys;
                    self.focus_sub_area = FocusSubArea::DeleteButton;
                },
                FocusArea::Hosts => {
                    self.focus_area = FocusArea::Groups;
                    self.focus_sub_area = FocusSubArea::DeleteButton;
                },
            }
        }
    }

    async fn connect_to_host(&mut self, host: Host) -> Result<()> {
        if self.ssh_client.is_connecting() || self.ssh_client.is_connected() {
            return Ok(());
        }

        // Find key path
        let key_path = if let Some(key_path) = &host.key_path {
            key_path.clone()
        } else if let Some(default_key) = self.config.get_default_key() {
            default_key.path.clone()
        } else {
            self.set_message("No SSH key configured for this host".to_string(), MessageType::Error);
            return Ok(());
        };

        // Create SSH event channel
        let (tx, rx) = mpsc::unbounded_channel();
        self.ssh_event_receiver = Some(rx);

        // Get terminal panel size for PTY
        let (width, height) = self.terminal_panel.get_size();

        // Start SSH connection
        self.ssh_client.connect(host.clone(), &key_path, tx, width, height).await?;
        
        self.set_message(
            format!("Connecting to {}@{}...", host.user, host.host),
            MessageType::Info
        );

        Ok(())
    }

    async fn handle_ssh_events(&mut self) {
        let mut events_to_process = Vec::new();
        
        // Collect events first to avoid borrowing issues
        if let Some(receiver) = &mut self.ssh_event_receiver {
            while let Ok(event) = receiver.try_recv() {
                events_to_process.push(event);
            }
        }
        
        // Process collected events
        let mut should_clear_receiver = false;
        for event in events_to_process {
            match &event {
                SshEvent::Data(data) => {
                    // Feed SSH data directly to the raw terminal panel
                    self.terminal_panel.write_ssh_data(data);
                },
                SshEvent::Connected { host } => {
                    self.set_message(
                        format!("Connected to {}", host.name),
                        MessageType::Success
                    );
                    self.terminal_panel.set_active(true);
                    self.ssh_client.connected = true;
                    self.ssh_client.connecting = false;
                },
                SshEvent::Disconnected => {
                    self.set_message("SSH connection closed".to_string(), MessageType::Info);
                    self.terminal_panel.set_active(false);
                    should_clear_receiver = true;
                },
                SshEvent::Error(err) => {
                    self.set_message(
                        format!("SSH error: {}", err),
                        MessageType::Error
                    );
                    self.terminal_panel.set_active(false);
                    should_clear_receiver = true;
                },
            }
            
            self.ssh_client.handle_event(event);
        }
        
        if should_clear_receiver {
            self.ssh_event_receiver = None;
        }
    }

    async fn send_ssh_input(&self, data: &[u8]) -> Result<()> {
        self.ssh_client.send_input(data).await
    }

    fn update_layout(&mut self, terminal_size: (u16, u16)) {
        self.terminal_size = terminal_size;
        
        // Calculate terminal panel bounds (right side of screen)
        let sidebar_width = terminal_size.0 / 3;
        let terminal_bounds = Rect {
            x: sidebar_width,
            y: 2,
            width: terminal_size.0 - sidebar_width - 1,
            height: terminal_size.1 - 6, // Account for title, message, and help
        };
        
        self.terminal_panel.set_bounds(terminal_bounds);
        
        // Resize SSH PTY if connected
        if self.ssh_client.is_connected() {
            let (width, height) = self.terminal_panel.get_size();
            tokio::spawn(async move {
                // Note: In a real implementation, you'd want to keep a reference to send this resize
                // For now, this is a placeholder to show the concept
            });
        }
    }
    
    async fn handle_add_button_press(&mut self) {
        match self.focus_area {
            FocusArea::Keys => {
                let form = KeyEditForm {
                    name: "New SSH Key".to_string(),
                    path: "~/.ssh/id_rsa".to_string(),
                    is_default: self.config.keys.is_empty(),
                    field_focus: 0,
                };
                self.modal_state = ModalState::AddKey(form);
            },
            FocusArea::Groups => {
                let form = GroupEditForm {
                    name: "New Group".to_string(),
                    color: "green".to_string(),
                    field_focus: 0,
                };
                self.modal_state = ModalState::AddGroup(form);
            },
            FocusArea::Hosts => {
                if self.selected_group > 0 && self.selected_group < self.config.groups.len() {
                    // Find default key index
                    let default_key_index = self.config.keys.iter()
                        .position(|k| k.is_default)
                        .unwrap_or(0);
                    
                    let form = HostEditForm {
                        name: "New Host".to_string(),
                        host: "example.com".to_string(),
                        port: "22".to_string(),
                        user: "user".to_string(),
                        key_path: String::new(),
                        use_key_selector: !self.config.keys.is_empty(), // Use selector if keys available
                        selected_key_index: default_key_index,
                        field_focus: 0,
                    };
                    self.modal_state = ModalState::AddHost(form);
                } else {
                    self.set_message("Cannot add hosts to 'All' group. Select a specific group first.".to_string(), MessageType::Error);
                }
            },
        }
    }
    
    async fn handle_edit_button_press(&mut self) {
        match self.focus_area {
            FocusArea::Keys => {
                if !self.config.keys.is_empty() && self.selected_key < self.config.keys.len() {
                    let key = &self.config.keys[self.selected_key];
                    let form = KeyEditForm {
                        name: key.name.clone(),
                        path: key.path.clone(),
                        is_default: key.is_default,
                        field_focus: 0,
                    };
                    self.modal_state = ModalState::EditKey(self.selected_key, form);
                }
            },
            FocusArea::Groups => {
                if self.config.groups.len() > 1 && self.selected_group < self.config.groups.len() && self.selected_group > 0 {
                    let group = &self.config.groups[self.selected_group];
                    let form = GroupEditForm {
                        name: group.name.clone(),
                        color: group.color.clone(),
                        field_focus: 0,
                    };
                    self.modal_state = ModalState::EditGroup(self.selected_group, form);
                } else {
                    self.set_message("Cannot edit the 'All' group.".to_string(), MessageType::Error);
                }
            },
            FocusArea::Hosts => {
                let hosts = self.config.get_hosts_for_group(self.selected_group);
                if !hosts.is_empty() && self.selected_host < hosts.len() && self.selected_group > 0 {
                    let host = &hosts[self.selected_host];
                    
                    // Try to find the key index if host has a specific key path
                    let (use_selector, selected_key_index) = if let Some(key_path) = &host.key_path {
                        let key_index = self.config.keys.iter()
                            .position(|k| &k.path == key_path)
                            .unwrap_or(0);
                        (true, key_index)
                    } else {
                        // Use default key
                        let default_key_index = self.config.keys.iter()
                            .position(|k| k.is_default)
                            .unwrap_or(0);
                        (true, default_key_index)
                    };
                    
                    let form = HostEditForm {
                        name: host.name.clone(),
                        host: host.host.clone(),
                        port: host.port.to_string(),
                        user: host.user.clone(),
                        key_path: host.key_path.as_ref().unwrap_or(&String::new()).clone(),
                        use_key_selector: use_selector && !self.config.keys.is_empty(),
                        selected_key_index,
                        field_focus: 0,
                    };
                    self.modal_state = ModalState::EditHost(self.selected_host, form);
                }
            },
        }
    }
    
    async fn handle_delete_button_press(&mut self) {
        match self.focus_area {
            FocusArea::Keys => {
                if !self.config.keys.is_empty() && self.selected_key < self.config.keys.len() {
                    let key_name = self.config.keys[self.selected_key].name.clone();
                    self.config.remove_key(&key_name);
                    // Adjust selection if necessary
                    if self.selected_key > 0 && self.selected_key >= self.config.keys.len() {
                        self.selected_key = self.config.keys.len() - 1;
                    }
                    self.set_message(format!("SSH key '{}' deleted.", key_name), MessageType::Success);
                    let _ = self.config.save(); // Save changes
                }
            },
            FocusArea::Groups => {
                if self.config.groups.len() > 1 && self.selected_group < self.config.groups.len() && self.selected_group > 0 {
                    let group_name = self.config.groups[self.selected_group].name.clone();
                    self.config.remove_group(&group_name);
                    // Adjust selection if necessary
                    if self.selected_group > 0 && self.selected_group >= self.config.groups.len() {
                        self.selected_group = self.config.groups.len() - 1;
                    }
                    self.selected_host = 0; // Reset host selection
                    self.set_message(format!("Group '{}' deleted.", group_name), MessageType::Success);
                    let _ = self.config.save(); // Save changes
                } else {
                    self.set_message("Cannot delete the 'All' group.".to_string(), MessageType::Error);
                }
            },
            FocusArea::Hosts => {
                let hosts = self.config.get_hosts_for_group(self.selected_group);
                if !hosts.is_empty() && self.selected_host < hosts.len() && self.selected_group > 0 {
                    let host_name = hosts[self.selected_host].name.clone();
                    let group_name = self.config.groups[self.selected_group].name.clone();
                    if let Ok(()) = self.config.remove_host(&group_name, &host_name) {
                        // Adjust selection if necessary
                        if self.selected_host > 0 && self.selected_host >= hosts.len() - 1 {
                            self.selected_host = hosts.len().saturating_sub(2);
                        }
                        self.set_message(format!("Host '{}' deleted from group '{}'.", host_name, group_name), MessageType::Success);
                        let _ = self.config.save(); // Save changes
                    } else {
                        self.set_message("Failed to delete host".to_string(), MessageType::Error);
                    }
                } else {
                    self.set_message("Cannot delete hosts from 'All' group.".to_string(), MessageType::Error);
                }
            },
        }
    }
    
    async fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        match mouse.kind {
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let col = mouse.column;
                let row = mouse.row;
                
                // First check if a modal is active - if so, handle modal clicks
                if !matches!(self.modal_state, ModalState::None) {
                    self.handle_modal_mouse_click(col, row);
                    return;
                }
                
                // Calculate sidebar width based on current terminal size
                let sidebar_width = self.terminal_size.0 / 3;
                
                // Check if click is in the sidebar (left third)
                if col < sidebar_width {
                    self.handle_sidebar_click(col, row);
                } else {
                    // Click is in the terminal panel area
                    if self.ssh_client.is_connected() {
                        // For now, just focus on the terminal when clicked
                        // In the future, we could send mouse events to SSH if the remote supports it
                        self.focus_area = FocusArea::Hosts; // Keep current focus structure
                    }
                }
            },
            MouseEventKind::ScrollUp => {
                // Handle scroll up in lists
                match self.focus_area {
                    FocusArea::Keys => {
                        if self.selected_key > 0 {
                            self.selected_key -= 1;
                        }
                    },
                    FocusArea::Groups => {
                        if self.selected_group > 0 {
                            self.selected_group -= 1;
                            self.selected_host = 0;
                        }
                    },
                    FocusArea::Hosts => {
                        if self.selected_host > 0 {
                            self.selected_host -= 1;
                        }
                    },
                }
            },
            MouseEventKind::ScrollDown => {
                // Handle scroll down in lists
                match self.focus_area {
                    FocusArea::Keys => {
                        if self.selected_key < self.config.keys.len().saturating_sub(1) {
                            self.selected_key += 1;
                        }
                    },
                    FocusArea::Groups => {
                        if self.selected_group < self.config.groups.len().saturating_sub(1) {
                            self.selected_group += 1;
                            self.selected_host = 0;
                        }
                    },
                    FocusArea::Hosts => {
                        let hosts = self.config.get_hosts_for_group(self.selected_group);
                        if self.selected_host < hosts.len().saturating_sub(1) {
                            self.selected_host += 1;
                        }
                    },
                }
            },
            _ => {}
        }
    }
    
    fn handle_sidebar_click(&mut self, col: u16, row: u16) {
        // The UI layout from ui.rs:
        // - Title bar is at row 0-1
        // - Keys panel starts around row 2
        // - Groups panel starts after keys
        // - Hosts panel starts after groups
        // - Buttons are at the bottom of each panel
        
        // This is a simplified mouse handling - in a real implementation,
        // you'd want to get the exact coordinates from the UI rendering
        let sidebar_height = self.terminal_size.1;
        let panel_height = (sidebar_height - 6) / 3; // Rough estimate, accounting for borders and message area
        
        // Determine which panel was clicked based on row
        if row >= 2 && row < 2 + panel_height {
            // Keys panel
            self.focus_area = FocusArea::Keys;
            let relative_row = row - 2;
            
            // Check if it's a button click (last few rows of the panel)
            if relative_row >= panel_height.saturating_sub(4) {
                // Button area - focus on the button (actions are handled separately)
                if col >= 2 && col <= 8 {
                    self.focus_sub_area = FocusSubArea::AddButton;
                } else if col >= 10 && col <= 16 {
                    self.focus_sub_area = FocusSubArea::EditButton;
                } else if col >= 18 && col <= 24 {
                    self.focus_sub_area = FocusSubArea::DeleteButton;
                }
            } else {
                // List area - select item based on row
                self.focus_sub_area = FocusSubArea::Items;
                let item_row = relative_row.saturating_sub(2); // Account for panel border
                if item_row < self.config.keys.len() as u16 {
                    self.selected_key = item_row as usize;
                }
            }
        } else if row >= 2 + panel_height && row < 2 + 2 * panel_height {
            // Groups panel
            self.focus_area = FocusArea::Groups;
            let relative_row = row - (2 + panel_height);
            
            if relative_row >= panel_height.saturating_sub(4) {
                // Button area
                if col >= 2 && col <= 8 {
                    self.focus_sub_area = FocusSubArea::AddButton;
                } else if col >= 10 && col <= 16 {
                    self.focus_sub_area = FocusSubArea::EditButton;
                } else if col >= 18 && col <= 24 {
                    self.focus_sub_area = FocusSubArea::DeleteButton;
                }
            } else {
                // List area
                self.focus_sub_area = FocusSubArea::Items;
                let item_row = relative_row.saturating_sub(2);
                if item_row < self.config.groups.len() as u16 {
                    self.selected_group = item_row as usize;
                    self.selected_host = 0; // Reset host selection when group changes
                }
            }
        } else if row >= 2 + 2 * panel_height {
            // Hosts panel
            self.focus_area = FocusArea::Hosts;
            let relative_row = row - (2 + 2 * panel_height);
            
            if relative_row >= panel_height.saturating_sub(4) {
                // Button area
                if col >= 2 && col <= 8 {
                    self.focus_sub_area = FocusSubArea::AddButton;
                } else if col >= 10 && col <= 16 {
                    self.focus_sub_area = FocusSubArea::EditButton;
                } else if col >= 18 && col <= 24 {
                    self.focus_sub_area = FocusSubArea::DeleteButton;
                }
            } else {
                // List area
                self.focus_sub_area = FocusSubArea::Items;
                let item_row = relative_row.saturating_sub(2);
                let hosts = self.config.get_hosts_for_group(self.selected_group);
                if item_row < hosts.len() as u16 {
                    self.selected_host = item_row as usize;
                }
            }
        }
    }
    
    fn handle_modal_mouse_click(&mut self, col: u16, row: u16) {
        // This is a simplified modal click handler
        // In a real implementation, you'd calculate the exact modal bounds
        let center_x = self.terminal_size.0 / 2;
        let center_y = self.terminal_size.1 / 2;
        
        // Check if click is outside modal bounds - if so, close modal
        if col < center_x.saturating_sub(30) || col > center_x + 30 ||
           row < center_y.saturating_sub(8) || row > center_y + 8 {
            self.modal_state = ModalState::None;
            return;
        }
        
        // TODO: Handle clicks on modal fields and buttons
        // This would require more precise coordinate calculations
        // based on the modal layout in modal.rs
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    
    // Create app state
    let mut app = AppState::new()?;
    
    // Main event loop
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(16); // ~60 FPS
    
    loop {
        // Handle SSH events
        app.handle_ssh_events().await;
        
        // Handle terminal events
        if event::poll(Duration::from_millis(1))? {
            match event::read()? {
                Event::Key(key) => {
                    // Check if modal is active and handle modal events first
                    if app.handle_modal_key_event(key.code, key.modifiers) {
                        continue; // Modal handled the event
                    }
                    
                    match (key.code, key.modifiers) {
                        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                            if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(b"\x03").await;
                            } else {
                                break;
                            }
                        },
                        (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                            if app.ssh_client.is_connected() {
                                let _ = app.ssh_client.disconnect().await;
                            } else {
                                break;
                            }
                        },
                        (KeyCode::Tab, KeyModifiers::NONE) => {
                            app.advance_focus(true);
                        },
                        (KeyCode::BackTab, _) => {
                            app.advance_focus(false);
                        },
                        (KeyCode::Up, _) => {
                            if app.focus_sub_area == FocusSubArea::Items {
                                match app.focus_area {
                                    FocusArea::Keys => {
                                        if app.selected_key > 0 {
                                            app.selected_key -= 1;
                                        }
                                    },
                                    FocusArea::Groups => {
                                        if app.selected_group > 0 {
                                            app.selected_group -= 1;
                                            app.selected_host = 0;
                                        }
                                    },
                                    FocusArea::Hosts => {
                                        if app.selected_host > 0 {
                                            app.selected_host -= 1;
                                        }
                                    },
                                }
                            } else if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(b"\x1b[A").await;
                            }
                        },
                        (KeyCode::Down, _) => {
                            if app.focus_sub_area == FocusSubArea::Items {
                                match app.focus_area {
                                    FocusArea::Keys => {
                                        if app.selected_key < app.config.keys.len().saturating_sub(1) {
                                            app.selected_key += 1;
                                        }
                                    },
                                    FocusArea::Groups => {
                                        if app.selected_group < app.config.groups.len().saturating_sub(1) {
                                            app.selected_group += 1;
                                            app.selected_host = 0;
                                        }
                                    },
                                    FocusArea::Hosts => {
                                        let hosts = app.config.get_hosts_for_group(app.selected_group);
                                        if app.selected_host < hosts.len().saturating_sub(1) {
                                            app.selected_host += 1;
                                        }
                                    },
                                }
                            } else if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(b"\x1b[B").await;
                            }
                        },
                        (KeyCode::Left, _) => {
                            if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(b"\x1b[D").await;
                            }
                        },
                        (KeyCode::Right, _) => {
                            if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(b"\x1b[C").await;
                            }
                        },
                        (KeyCode::Enter, _) => {
                            if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(b"\r").await;
                            } else {
                                match app.focus_sub_area {
                                    FocusSubArea::Items => {
                                        if app.focus_area == FocusArea::Hosts {
                                            let hosts = app.config.get_hosts_for_group(app.selected_group);
                                            if let Some(host) = hosts.get(app.selected_host) {
                                                let _ = app.connect_to_host(host.clone()).await;
                                            }
                                        }
                                    },
                                    FocusSubArea::AddButton => {
                                        app.handle_add_button_press().await;
                                    },
                                    FocusSubArea::EditButton => {
                                        app.handle_edit_button_press().await;
                                    },
                                    FocusSubArea::DeleteButton => {
                                        app.handle_delete_button_press().await;
                                    },
                                }
                            }
                        },
                        (KeyCode::Backspace, _) => {
                            if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(b"\x7f").await;
                            }
                        },
                        (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                            if !app.ssh_client.is_connected() {
                                // Ctrl+N: Add new item in current panel
                                app.handle_add_button_press().await;
                            }
                        },
                        (KeyCode::Char(c), _) => {
                            if app.ssh_client.is_connected() {
                                let _ = app.send_ssh_input(&[c as u8]).await;
                            }
                        },
                        _ => {}
                    }
                },
                Event::Resize(width, height) => {
                    app.update_layout((width, height));
                },
                Event::Mouse(mouse) => {
                    // Store the previous focus state to detect button clicks
                    let prev_focus_area = app.focus_area;
                    let prev_focus_sub_area = app.focus_sub_area;
                    
                    app.handle_mouse_event(mouse).await;
                    
                    // If we clicked on a button (focus changed to a button), execute its action
                    if matches!(mouse.kind, MouseEventKind::Down(crossterm::event::MouseButton::Left)) &&
                       matches!(app.focus_sub_area, FocusSubArea::AddButton | FocusSubArea::EditButton | FocusSubArea::DeleteButton) &&
                       (prev_focus_area != app.focus_area || prev_focus_sub_area != app.focus_sub_area) {
                        
                        match app.focus_sub_area {
                            FocusSubArea::AddButton => {
                                app.handle_add_button_press().await;
                            },
                            FocusSubArea::EditButton => {
                                app.handle_edit_button_press().await;
                            },
                            FocusSubArea::DeleteButton => {
                                app.handle_delete_button_press().await;
                            },
                            _ => {}
                        }
                    }
                },
                _ => {}
            }
        }
        
        // Render UI
        terminal.draw(|frame| {
            ui::render(frame, &mut app);
        })?;
        
        // Control frame rate
        let now = Instant::now();
        if now.duration_since(last_tick) >= tick_rate {
            last_tick = now;
        }
    }
    
    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, crossterm::event::DisableMouseCapture)?;
    
    Ok(())
}
