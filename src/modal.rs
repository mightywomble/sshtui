use crate::{AppState, ModalState, KeyEditForm, GroupEditForm, HostEditForm, ConfirmAction, MessageType};
use crate::config::{SshKey, Group, Host};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph, List, ListItem},
};

impl AppState {
    pub fn handle_modal_key_event(&mut self, key: KeyCode, modifiers: KeyModifiers) -> bool {
        if let ModalState::None = self.modal_state {
            return false; // Not handled
        }

        match (key, modifiers) {
            (KeyCode::Esc, _) => {
                self.modal_state = ModalState::None;
                true
            },
            (KeyCode::Enter, _) => {
                self.handle_modal_submit();
                true
            },
            (KeyCode::Tab, _) => {
                self.advance_modal_field(true);
                true
            },
            (KeyCode::BackTab, _) => {
                self.advance_modal_field(false);
                true
            },
            (KeyCode::Up, _) => {
                self.advance_modal_field(false);
                true
            },
            (KeyCode::Down, _) => {
                self.advance_modal_field(true);
                true
            },
            (KeyCode::Char(c), _) => {
                self.handle_modal_char_input(c);
                true
            },
            (KeyCode::Backspace, _) => {
                self.handle_modal_backspace();
                true
            },
            _ => false
        }
    }

    fn advance_modal_field(&mut self, forward: bool) {
        match &mut self.modal_state {
            ModalState::AddKey(form) | ModalState::EditKey(_, form) => {
                let max_fields = 3;
                if forward {
                    form.field_focus = (form.field_focus + 1) % max_fields;
                } else {
                    form.field_focus = if form.field_focus == 0 { max_fields - 1 } else { form.field_focus - 1 };
                }
            },
            ModalState::AddGroup(form) | ModalState::EditGroup(_, form) => {
                let max_fields = 2;
                if forward {
                    form.field_focus = (form.field_focus + 1) % max_fields;
                } else {
                    form.field_focus = if form.field_focus == 0 { max_fields - 1 } else { form.field_focus - 1 };
                }
            },
            ModalState::AddHost(form) | ModalState::EditHost(_, form) => {
                let max_fields = 5;
                if forward {
                    form.field_focus = (form.field_focus + 1) % max_fields;
                } else {
                    form.field_focus = if form.field_focus == 0 { max_fields - 1 } else { form.field_focus - 1 };
                }
            },
            _ => {}
        }
    }

    fn handle_modal_char_input(&mut self, c: char) {
        match &mut self.modal_state {
            ModalState::AddKey(form) | ModalState::EditKey(_, form) => {
                match form.field_focus {
                    0 => form.name.push(c),
                    1 => form.path.push(c),
                    2 => {
                        if c == 'y' || c == 'Y' || c == 't' || c == 'T' {
                            form.is_default = true;
                        } else if c == 'n' || c == 'N' || c == 'f' || c == 'F' {
                            form.is_default = false;
                        }
                    },
                    _ => {}
                }
            },
            ModalState::AddGroup(form) | ModalState::EditGroup(_, form) => {
                match form.field_focus {
                    0 => form.name.push(c),
                    1 => form.color.push(c),
                    _ => {}
                }
            },
            ModalState::AddHost(form) | ModalState::EditHost(_, form) => {
                match form.field_focus {
                    0 => form.name.push(c),
                    1 => form.host.push(c),
                    2 => {
                        if c.is_ascii_digit() {
                            form.port.push(c);
                        }
                    },
                    3 => form.user.push(c),
                    4 => {
                        if form.use_key_selector {
                            // In key selector mode, handle selection
                            match c {
                                '↑' | 'k' => {
                                    if form.selected_key_index > 0 {
                                        form.selected_key_index -= 1;
                                    }
                                },
                                '↓' | 'j' => {
                                    // Bound check against available keys
                                    if form.selected_key_index + 1 < self.config.keys.len() {
                                        form.selected_key_index += 1;
                                    }
                                },
                                's' | 'S' => {
                                    // Switch to manual key path input
                                    form.use_key_selector = false;
                                }
                                _ => {} // Ignore other characters in selector mode
                            }
                        } else {
                            // Manual key path input
                            match c {
                                's' | 'S' => {
                                    // Switch back to key selector
                                    form.use_key_selector = true;
                                }
                                _ => form.key_path.push(c),
                            }
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    fn handle_modal_backspace(&mut self) {
        match &mut self.modal_state {
            ModalState::AddKey(form) | ModalState::EditKey(_, form) => {
                match form.field_focus {
                    0 => { form.name.pop(); },
                    1 => { form.path.pop(); },
                    2 => {}, // Boolean field, no backspace
                    _ => {}
                }
            },
            ModalState::AddGroup(form) | ModalState::EditGroup(_, form) => {
                match form.field_focus {
                    0 => { form.name.pop(); },
                    1 => { form.color.pop(); },
                    _ => {}
                }
            },
            ModalState::AddHost(form) | ModalState::EditHost(_, form) => {
                match form.field_focus {
                    0 => { form.name.pop(); },
                    1 => { form.host.pop(); },
                    2 => { form.port.pop(); },
                    3 => { form.user.pop(); },
                    4 => {
                        // Only allow backspace in manual key path input mode
                        if !form.use_key_selector {
                            form.key_path.pop();
                        }
                    },
                    _ => {}
                }
            },
            _ => {}
        }
    }

    fn handle_modal_submit(&mut self) {
        match self.modal_state.clone() {
            ModalState::AddKey(form) => {
                if form.name.trim().is_empty() {
                    self.set_message("Key name cannot be empty".to_string(), MessageType::Error);
                    return;
                }
                if form.path.trim().is_empty() {
                    self.set_message("Key path cannot be empty".to_string(), MessageType::Error);
                    return;
                }

                let new_key = SshKey {
                    name: form.name.trim().to_string(),
                    path: form.path.trim().to_string(),
                    is_default: form.is_default,
                };

                self.config.add_key(new_key);
                self.selected_key = self.config.keys.len() - 1;
                let _ = self.config.save();
                
                self.set_message("SSH key added successfully!".to_string(), MessageType::Success);
                self.modal_state = ModalState::None;
            },
            ModalState::EditKey(index, form) => {
                if index < self.config.keys.len() {
                    if form.name.trim().is_empty() {
                        self.set_message("Key name cannot be empty".to_string(), MessageType::Error);
                        return;
                    }
                    if form.path.trim().is_empty() {
                        self.set_message("Key path cannot be empty".to_string(), MessageType::Error);
                        return;
                    }

                    self.config.keys[index] = SshKey {
                        name: form.name.trim().to_string(),
                        path: form.path.trim().to_string(),
                        is_default: form.is_default,
                    };
                    
                    let _ = self.config.save();
                    self.set_message("SSH key updated successfully!".to_string(), MessageType::Success);
                }
                self.modal_state = ModalState::None;
            },
            ModalState::AddGroup(form) => {
                if form.name.trim().is_empty() {
                    self.set_message("Group name cannot be empty".to_string(), MessageType::Error);
                    return;
                }

                let new_group = Group {
                    name: form.name.trim().to_string(),
                    color: if form.color.trim().is_empty() { "green".to_string() } else { form.color.trim().to_string() },
                    hosts: Vec::new(),
                };

                self.config.add_group(new_group);
                self.selected_group = self.config.groups.len() - 1;
                self.selected_host = 0;
                let _ = self.config.save();
                
                self.set_message("Group added successfully!".to_string(), MessageType::Success);
                self.modal_state = ModalState::None;
            },
            ModalState::EditGroup(index, form) => {
                if index < self.config.groups.len() && index > 0 {
                    if form.name.trim().is_empty() {
                        self.set_message("Group name cannot be empty".to_string(), MessageType::Error);
                        return;
                    }

                    self.config.groups[index].name = form.name.trim().to_string();
                    self.config.groups[index].color = if form.color.trim().is_empty() { "green".to_string() } else { form.color.trim().to_string() };
                    
                    let _ = self.config.save();
                    self.set_message("Group updated successfully!".to_string(), MessageType::Success);
                }
                self.modal_state = ModalState::None;
            },
            ModalState::AddHost(form) => {
                if form.name.trim().is_empty() {
                    self.set_message("Host name cannot be empty".to_string(), MessageType::Error);
                    return;
                }
                if form.host.trim().is_empty() {
                    self.set_message("Host address cannot be empty".to_string(), MessageType::Error);
                    return;
                }
                if form.user.trim().is_empty() {
                    self.set_message("Username cannot be empty".to_string(), MessageType::Error);
                    return;
                }

                let port = form.port.parse::<u16>().unwrap_or(22);
                let key_path = if form.use_key_selector {
                    // Use selected key from dropdown
                    if form.selected_key_index < self.config.keys.len() {
                        Some(self.config.keys[form.selected_key_index].path.clone())
                    } else {
                        None
                    }
                } else {
                    // Use manual key path input
                    if form.key_path.trim().is_empty() { None } else { Some(form.key_path.trim().to_string()) }
                };

                let new_host = Host {
                    name: form.name.trim().to_string(),
                    host: form.host.trim().to_string(),
                    port,
                    user: form.user.trim().to_string(),
                    key_path,
                };

                if self.selected_group > 0 && self.selected_group < self.config.groups.len() {
                    let group_name = self.config.groups[self.selected_group].name.clone();
                    if let Ok(()) = self.config.add_host_to_group(&group_name, new_host) {
                        let hosts = self.config.get_hosts_for_group(self.selected_group);
                        self.selected_host = hosts.len() - 1;
                        let _ = self.config.save();
                        self.set_message("Host added successfully!".to_string(), MessageType::Success);
                    } else {
                        self.set_message("Failed to add host to group".to_string(), MessageType::Error);
                    }
                }
                self.modal_state = ModalState::None;
            },
            ModalState::EditHost(index, form) => {
                let hosts = self.config.get_hosts_for_group(self.selected_group);
                if index < hosts.len() && self.selected_group > 0 {
                    if form.name.trim().is_empty() {
                        self.set_message("Host name cannot be empty".to_string(), MessageType::Error);
                        return;
                    }
                    if form.host.trim().is_empty() {
                        self.set_message("Host address cannot be empty".to_string(), MessageType::Error);
                        return;
                    }
                    if form.user.trim().is_empty() {
                        self.set_message("Username cannot be empty".to_string(), MessageType::Error);
                        return;
                    }

                    let port = form.port.parse::<u16>().unwrap_or(22);
                    let key_path = if form.use_key_selector {
                        // Use selected key from dropdown
                        if form.selected_key_index < self.config.keys.len() {
                            Some(self.config.keys[form.selected_key_index].path.clone())
                        } else {
                            None
                        }
                    } else {
                        // Use manual key path input
                        if form.key_path.trim().is_empty() { None } else { Some(form.key_path.trim().to_string()) }
                    };

                    let updated_host = Host {
                        name: form.name.trim().to_string(),
                        host: form.host.trim().to_string(),
                        port,
                        user: form.user.trim().to_string(),
                        key_path,
                    };

                    let group_name = self.config.groups[self.selected_group].name.clone();
                    let old_host_name = hosts[index].name.clone();
                    
                    // Remove old host and add updated one
                    if let Ok(()) = self.config.remove_host(&group_name, &old_host_name) {
                        if let Ok(()) = self.config.add_host_to_group(&group_name, updated_host) {
                            let _ = self.config.save();
                            self.set_message("Host updated successfully!".to_string(), MessageType::Success);
                        } else {
                            self.set_message("Failed to update host".to_string(), MessageType::Error);
                        }
                    } else {
                        self.set_message("Failed to update host".to_string(), MessageType::Error);
                    }
                }
                self.modal_state = ModalState::None;
            },
            ModalState::Confirm(_, action) => {
                match action {
                    ConfirmAction::DeleteKey(index) => {
                        if index < self.config.keys.len() {
                            let key_name = self.config.keys[index].name.clone();
                            self.config.remove_key(&key_name);
                            if self.selected_key >= self.config.keys.len() && self.selected_key > 0 {
                                self.selected_key = self.config.keys.len() - 1;
                            }
                            let _ = self.config.save();
                            self.set_message(format!("SSH key '{}' deleted", key_name), MessageType::Success);
                        }
                    },
                    ConfirmAction::DeleteGroup(index) => {
                        if index < self.config.groups.len() && index > 0 {
                            let group_name = self.config.groups[index].name.clone();
                            self.config.remove_group(&group_name);
                            if self.selected_group >= self.config.groups.len() && self.selected_group > 0 {
                                self.selected_group = self.config.groups.len() - 1;
                            }
                            self.selected_host = 0;
                            let _ = self.config.save();
                            self.set_message(format!("Group '{}' deleted", group_name), MessageType::Success);
                        }
                    },
                    ConfirmAction::DeleteHost(index) => {
                        let hosts = self.config.get_hosts_for_group(self.selected_group);
                        if index < hosts.len() && self.selected_group > 0 {
                            let host_name = hosts[index].name.clone();
                            let group_name = self.config.groups[self.selected_group].name.clone();
                            if let Ok(()) = self.config.remove_host(&group_name, &host_name) {
                                if self.selected_host >= hosts.len() - 1 && self.selected_host > 0 {
                                    self.selected_host = hosts.len() - 2;
                                }
                                let _ = self.config.save();
                                self.set_message(format!("Host '{}' deleted", host_name), MessageType::Success);
                            }
                        }
                    },
                }
                self.modal_state = ModalState::None;
            },
            _ => {}
        }
    }
}

pub fn render_modal(frame: &mut Frame, app: &AppState) {
    match &app.modal_state {
        ModalState::AddKey(form) => render_key_modal(frame, "Add SSH Key", form, true),
        ModalState::EditKey(_, form) => render_key_modal(frame, "Edit SSH Key", form, false),
        ModalState::AddGroup(form) => render_group_modal(frame, "Add Group", form, true),
        ModalState::EditGroup(_, form) => render_group_modal(frame, "Edit Group", form, false),
        ModalState::AddHost(form) => render_host_modal(frame, "Add Host", form, &app.config.keys, true),
        ModalState::EditHost(_, form) => render_host_modal(frame, "Edit Host", form, &app.config.keys, false),
        ModalState::Confirm(message, _) => render_confirm_modal(frame, message),
        ModalState::None => {}
    }
}

fn render_key_modal(frame: &mut Frame, title: &str, form: &KeyEditForm, _is_add: bool) {
    let area = centered_rect(60, 12, frame.size());
    
    // Clear the area
    frame.render_widget(Clear, area);
    
    // Render modal background
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, area);
    
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Name label
            Constraint::Length(1), // Name input
            Constraint::Length(1), // Path label
            Constraint::Length(1), // Path input
            Constraint::Length(1), // Default label
            Constraint::Length(1), // Default input
            Constraint::Length(1), // Empty
            Constraint::Length(1), // Help text
        ])
        .split(area);
    
    // Name field
    let name_style = if form.field_focus == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new("Name:").style(name_style), inner[0]);
    let name_input = Paragraph::new(form.name.as_str())
        .style(if form.field_focus == 0 {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().bg(Color::Gray).fg(Color::Black)
        });
    frame.render_widget(name_input, inner[1]);
    
    // Path field
    let path_style = if form.field_focus == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new("Path:").style(path_style), inner[2]);
    let path_input = Paragraph::new(form.path.as_str())
        .style(if form.field_focus == 1 {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().bg(Color::Gray).fg(Color::Black)
        });
    frame.render_widget(path_input, inner[3]);
    
    // Default field
    let default_style = if form.field_focus == 2 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new("Is Default:").style(default_style), inner[4]);
    let default_input = Paragraph::new(if form.is_default { "Yes" } else { "No" })
        .style(if form.field_focus == 2 {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().bg(Color::Gray).fg(Color::Black)
        });
    frame.render_widget(default_input, inner[5]);
    
    // Help text
    let help_text = "Tab/↑↓=navigate | Enter=save | Esc=cancel";
    frame.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        inner[7]
    );
}

fn render_group_modal(frame: &mut Frame, title: &str, form: &GroupEditForm, _is_add: bool) {
    let area = centered_rect(60, 10, frame.size());
    
    // Clear the area
    frame.render_widget(Clear, area);
    
    // Render modal background
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, area);
    
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Name label
            Constraint::Length(1), // Name input
            Constraint::Length(1), // Color label
            Constraint::Length(1), // Color input
            Constraint::Length(1), // Empty
            Constraint::Length(1), // Help text
        ])
        .split(area);
    
    // Name field
    let name_style = if form.field_focus == 0 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new("Name:").style(name_style), inner[0]);
    let name_input = Paragraph::new(form.name.as_str())
        .style(if form.field_focus == 0 {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().bg(Color::Gray).fg(Color::Black)
        });
    frame.render_widget(name_input, inner[1]);
    
    // Color field
    let color_style = if form.field_focus == 1 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    frame.render_widget(Paragraph::new("Color:").style(color_style), inner[2]);
    let color_input = Paragraph::new(form.color.as_str())
        .style(if form.field_focus == 1 {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().bg(Color::Gray).fg(Color::Black)
        });
    frame.render_widget(color_input, inner[3]);
    
    // Help text
    let help_text = "Tab/↑↓=navigate | Enter=save | Esc=cancel";
    frame.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        inner[5]
    );
}

fn render_host_modal(frame: &mut Frame, title: &str, form: &HostEditForm, keys: &[SshKey], _is_add: bool) {
    let area = centered_rect(70, 16, frame.size());
    
    // Clear the area
    frame.render_widget(Clear, area);
    
    // Render modal background
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, area);
    
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(1), // Name label
            Constraint::Length(1), // Name input
            Constraint::Length(1), // Host label
            Constraint::Length(1), // Host input
            Constraint::Length(1), // Port label
            Constraint::Length(1), // Port input
            Constraint::Length(1), // User label
            Constraint::Length(1), // User input
            Constraint::Length(1), // Key Path label
            Constraint::Length(1), // Key Path input
            Constraint::Length(1), // Empty
            Constraint::Length(1), // Help text
        ])
        .split(area);
    
    // Render regular fields (Name, Host, Port, User)
    let regular_fields = [
        ("Name:", &form.name),
        ("Host:", &form.host),
        ("Port:", &form.port),
        ("User:", &form.user),
    ];
    
    for (i, (label, value)) in regular_fields.iter().enumerate() {
        let label_style = if form.field_focus == i {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        frame.render_widget(Paragraph::new(*label).style(label_style), inner[i * 2]);
        
        let input_style = if form.field_focus == i {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().bg(Color::Gray).fg(Color::Black)
        };
        frame.render_widget(Paragraph::new(value.as_str()).style(input_style), inner[i * 2 + 1]);
    }
    
    // Render SSH Key field (field 4) - either selector or manual input
    let key_label_style = if form.field_focus == 4 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    
    let key_label = if form.use_key_selector {
        "SSH Key (s=manual):"
    } else {
        "Key Path (s=selector):"
    };
    frame.render_widget(Paragraph::new(key_label).style(key_label_style), inner[8]);
    
    if form.use_key_selector {
        // Show key selector dropdown
        let display_text = if form.selected_key_index < keys.len() {
            format!("▼ {}", keys[form.selected_key_index].name)
        } else {
            "▼ No keys available".to_string()
        };
        
        let input_style = if form.field_focus == 4 {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default().bg(Color::Gray).fg(Color::White)
        };
        frame.render_widget(Paragraph::new(display_text).style(input_style), inner[9]);
    } else {
        // Show manual key path input
        let input_style = if form.field_focus == 4 {
            Style::default().bg(Color::White).fg(Color::Black)
        } else {
            Style::default().bg(Color::Gray).fg(Color::Black)
        };
        frame.render_widget(Paragraph::new(form.key_path.as_str()).style(input_style), inner[9]);
    }
    
    // Help text
    let help_text = if form.use_key_selector && form.field_focus == 4 {
        "j/k/↑↓=select key | s=manual | Tab=next | Enter=save | Esc=cancel"
    } else {
        "Tab/↑↓=navigate | Enter=save | Esc=cancel"
    };
    frame.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        inner[11]
    );
}

fn render_confirm_modal(frame: &mut Frame, message: &str) {
    let area = centered_rect(50, 8, frame.size());
    
    // Clear the area
    frame.render_widget(Clear, area);
    
    // Render modal background
    let block = Block::default()
        .title("Confirm")
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::DarkGray));
    frame.render_widget(block, area);
    
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Min(1), // Message
            Constraint::Length(1), // Empty
            Constraint::Length(1), // Help text
        ])
        .split(area);
    
    frame.render_widget(
        Paragraph::new(message)
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true }),
        inner[0]
    );
    
    // Help text
    let help_text = "Enter=confirm | Esc=cancel";
    frame.render_widget(
        Paragraph::new(help_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center),
        inner[2]
    );
}

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((r.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
