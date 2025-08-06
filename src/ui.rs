use crate::{AppState, FocusArea, FocusSubArea, MessageType};
use crate::dashboard;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Clear},
};

pub fn render(frame: &mut Frame, app: &mut AppState) {
    let size = frame.size();
    
    // Update app layout based on current terminal size
    app.update_layout((size.width, size.height));
    
    // Main layout: Title at top, content in middle, message and help at bottom
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),     // Title
            Constraint::Min(0),        // Main content
            Constraint::Length(1),     // Message
            Constraint::Length(1),     // Help
        ])
        .split(size);
    
    // Render title
    let title = Paragraph::new("🦀 SSH TUI Manager (Rust)")
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    frame.render_widget(title, main_layout[0]);
    
    // Main content layout: Sidebar + Terminal panel
    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33), // Sidebar (keys, groups, hosts)
            Constraint::Percentage(67), // Terminal panel
        ])
        .split(main_layout[1]);
    
    // Render sidebar
    render_sidebar(frame, app, content_layout[0]);
    
    // Render terminal panel
    if app.ssh_client.is_connected() || app.ssh_client.is_connecting() {
        app.terminal_panel.render(frame);
    } else {
        // Render dashboard when not connected
        render_dashboard_panel(frame, app, content_layout[1]);
    }
    
    // Render message
    render_message(frame, app, main_layout[2]);
    
    // Render help
    render_help(frame, app, main_layout[3]);
    
    // Render modal if active
    crate::modal::render_modal(frame, app);
}

fn render_sidebar(frame: &mut Frame, app: &AppState, area: Rect) {
    // Split sidebar into three panels
    let sidebar_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // SSH Keys panel
            Constraint::Length(8),  // Groups panel
            Constraint::Min(0),     // Hosts panel
        ])
        .split(area);
    
    // Render SSH Keys panel
    render_keys_panel(frame, app, sidebar_layout[0]);
    
    // Render Groups panel
    render_groups_panel(frame, app, sidebar_layout[1]);
    
    // Render Hosts panel
    render_hosts_panel(frame, app, sidebar_layout[2]);
}

fn render_keys_panel(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.focus_area == FocusArea::Keys;
    
    let block = Block::default()
        .title("SSH Keys")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    if app.config.keys.is_empty() {
        let empty_msg = Paragraph::new("No SSH keys yet.\nPress Ctrl+N to add one.")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);
        frame.render_widget(empty_msg, inner);
    } else {
        let items: Vec<ListItem> = app.config.keys.iter().enumerate().map(|(i, key)| {
            let content = if key.is_default {
                format!("⭐ {}", key.name)
            } else {
                key.name.clone()
            };
            
            let style = if i == app.selected_key && is_focused && app.focus_sub_area == FocusSubArea::Items {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            
            ListItem::new(content).style(style)
        }).collect();
        
        let list = List::new(items);
        
        // Render list in most of the area, leaving space for buttons
        let list_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };
        
        frame.render_widget(list, list_area);
        
        // Render action buttons
        render_action_buttons(frame, app, FocusArea::Keys, inner);
    }
}

fn render_groups_panel(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.focus_area == FocusArea::Groups;
    
    let block = Block::default()
        .title("Groups")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let items: Vec<ListItem> = app.config.groups.iter().enumerate().map(|(i, group)| {
        let host_count = if i == 0 && group.name == "All" {
            // Count all hosts from real groups
            app.config.groups.iter().skip(1).map(|g| g.hosts.len()).sum()
        } else {
            group.hosts.len()
        };
        
        let content = format!("{} ({})", group.name, host_count);
        
        let style = if i == app.selected_group && is_focused && app.focus_sub_area == FocusSubArea::Items {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        };
        
        ListItem::new(content).style(style)
    }).collect();
    
    let list = List::new(items);
    
    // Render list in most of the area, leaving space for buttons
    let list_area = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height.saturating_sub(1),
    };
    
    frame.render_widget(list, list_area);
    
    // Render action buttons
    render_action_buttons(frame, app, FocusArea::Groups, inner);
}

fn render_hosts_panel(frame: &mut Frame, app: &AppState, area: Rect) {
    let is_focused = app.focus_area == FocusArea::Hosts;
    
    let block = Block::default()
        .title("Hosts")
        .borders(Borders::ALL)
        .border_style(if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        });
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    let hosts = app.config.get_hosts_for_group(app.selected_group);
    
    if hosts.is_empty() {
        let empty_msg = if app.selected_group == 0 && !app.config.groups.is_empty() && app.config.groups[0].name == "All" {
            Paragraph::new("No hosts in any group.\nAdd hosts to specific groups\nto see them here.")
        } else {
            Paragraph::new("No hosts in this group.\nPress [+] to add one.")
        }.style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
        
        frame.render_widget(empty_msg, inner);
    } else {
        let items: Vec<ListItem> = hosts.iter().enumerate().map(|(i, host)| {
            let content = format!("{}\n  {}@{}:{}", host.name, host.user, host.host, host.port);
            
            let style = if i == app.selected_host && is_focused && app.focus_sub_area == FocusSubArea::Items {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            
            ListItem::new(content).style(style)
        }).collect();
        
        let list = List::new(items);
        
        // Render list in most of the area, leaving space for buttons
        let list_area = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height.saturating_sub(1),
        };
        
        frame.render_widget(list, list_area);
        
        // Render action buttons
        render_action_buttons(frame, app, FocusArea::Hosts, inner);
    }
}

fn render_action_buttons(frame: &mut Frame, app: &AppState, panel_focus: FocusArea, area: Rect) {
    let is_panel_focused = app.focus_area == panel_focus;
    
    if !is_panel_focused {
        return; // Only show buttons for focused panel
    }
    
    // Button area is at the bottom of the panel
    let button_area = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    
    // Create button texts with focus highlighting
    let add_style = if app.focus_sub_area == FocusSubArea::AddButton {
        Style::default().bg(Color::Green).fg(Color::Black).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    
    let edit_style = if app.focus_sub_area == FocusSubArea::EditButton {
        Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Blue)
    };
    
    let delete_style = if app.focus_sub_area == FocusSubArea::DeleteButton {
        Style::default().bg(Color::Red).fg(Color::White).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };
    
    // Check if buttons should be enabled
    let (has_edit_items, has_delete_items) = match panel_focus {
        FocusArea::Keys => (!app.config.keys.is_empty(), !app.config.keys.is_empty()),
        FocusArea::Groups => (app.config.groups.len() > 1, app.config.groups.len() > 1),
        FocusArea::Hosts => {
            let hosts = app.config.get_hosts_for_group(app.selected_group);
            (!hosts.is_empty(), !hosts.is_empty())
        },
    };
    
    let edit_style = if has_edit_items { edit_style } else { Style::default().fg(Color::DarkGray) };
    let delete_style = if has_delete_items { delete_style } else { Style::default().fg(Color::DarkGray) };
    
    let buttons = Paragraph::new(
        Line::from(vec![
            Span::styled("[+]", add_style),
            Span::raw(" "),
            Span::styled("[E]", edit_style),
            Span::raw(" "),
            Span::styled("[D]", delete_style),
        ])
    );
    
    frame.render_widget(buttons, button_area);
}

fn render_dashboard_panel(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .title("🖥️ Dashboard")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Gray));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    // Render the colorful dashboard
    let dashboard_content = dashboard::render_dashboard(app, inner.width, inner.height);
    let dashboard_widget = Paragraph::new(dashboard_content)
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    frame.render_widget(dashboard_widget, inner);
}

fn render_message(frame: &mut Frame, app: &AppState, area: Rect) {
    if !app.message.is_empty() {
        let style = match app.message_type {
            MessageType::Success => Style::default().fg(Color::Green),
            MessageType::Error => Style::default().fg(Color::Red),
            MessageType::Info => Style::default().fg(Color::Yellow),
        };
        
        let message = Paragraph::new(app.message.as_str())
            .style(style)
            .alignment(Alignment::Center);
        
        frame.render_widget(message, area);
    }
}

fn render_help(frame: &mut Frame, app: &AppState, area: Rect) {
    let help_text = if app.ssh_client.is_connected() {
        "SSH Connected: Type to interact | Ctrl+Q=disconnect | All keys sent to remote host"
    } else {
        match app.focus_area {
            FocusArea::Keys => "Keys: ↑/↓=navigate | Tab=next panel | Enter=set default | [+/E/D] or Ctrl+N=add/edit/delete",
            FocusArea::Groups => "Groups: ↑/↓=navigate | Tab=next panel | [+/E/D] or Ctrl+N=add/edit/delete",
            FocusArea::Hosts => "Hosts: ↑/↓=navigate | Tab=next panel | Enter=connect | [+/E/D] or Ctrl+N=add/edit/delete",
        }
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    
    frame.render_widget(help, area);
}
