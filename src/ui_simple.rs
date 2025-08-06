use crate::{AppState, FocusArea};
use crate::dashboard;
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph},
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
    let title = Paragraph::new("🦀 SSH TUI Manager (Rust) - Demo")
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
    
    // Render dashboard panel
    render_dashboard_panel(frame, app, content_layout[1]);
    
    // Render message
    render_message(frame, app, main_layout[2]);
    
    // Render help
    render_help(frame, app, main_layout[3]);
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
        let empty_msg = Paragraph::new("No SSH keys yet.\nAdd some via config file.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(empty_msg, inner);
    } else {
        let items: Vec<ListItem> = app.config.keys.iter().enumerate().map(|(i, key)| {
            let content = if key.is_default {
                format!("⭐ {}", key.name)
            } else {
                key.name.clone()
            };
            
            let style = if i == app.selected_key && is_focused {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            
            ListItem::new(content).style(style)
        }).collect();
        
        let list = List::new(items);
        frame.render_widget(list, inner);
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
        
        let style = if i == app.selected_group && is_focused {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        };
        
        ListItem::new(content).style(style)
    }).collect();
    
    let list = List::new(items);
    frame.render_widget(list, inner);
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
            Paragraph::new("No hosts in any group.\nAdd hosts via config file.")
        } else {
            Paragraph::new("No hosts in this group.\nAdd hosts via config file.")
        }.style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
        
        frame.render_widget(empty_msg, inner);
    } else {
        let items: Vec<ListItem> = hosts.iter().enumerate().map(|(i, host)| {
            let content = format!("{}\n  {}@{}:{}", host.name, host.user, host.host, host.port);
            
            let style = if i == app.selected_host && is_focused {
                Style::default().bg(Color::Blue).fg(Color::White)
            } else {
                Style::default()
            };
            
            ListItem::new(content).style(style)
        }).collect();
        
        let list = List::new(items);
        frame.render_widget(list, inner);
    }
}

fn render_dashboard_panel(frame: &mut Frame, app: &AppState, area: Rect) {
    let block = Block::default()
        .title("🖥️ Dashboard - Raw SSH Terminal Demo")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    
    let inner = block.inner(area);
    frame.render_widget(block, area);
    
    // Create demo content
    let content = vec![
        Line::from(vec![
            Span::styled(
                "🚀 SSH TUI Manager (Rust Edition)",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            )
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "⚡ KEY INNOVATION:",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            )
        ]),
        Line::from(vec![
            Span::styled(
                "Raw SSH terminal WITHIN the panel!",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            )
        ]),
        Line::from(""),
        Line::from("Unlike the Go version which needs separate modes:"),
        Line::from("• Go: Standard → Fullscreen → Raw (mode switching)"),
        Line::from("• Rust: Raw terminal directly in-panel (no switching!)"),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "🎯 This means:",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
            )
        ]),
        Line::from("• vim works perfectly in the panel"),
        Line::from("• htop displays correctly in the panel"),
        Line::from("• tmux sessions run seamlessly in the panel"),
        Line::from("• Sidebar stays visible while using TUI apps"),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Navigate: TAB/Shift+TAB | Select: ↑/↓ | Connect: ENTER",
                Style::default().fg(Color::DarkGray)
            )
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "(This is a demo - SSH connection not implemented)",
                Style::default().fg(Color::Red).add_modifier(Modifier::ITALIC)
            )
        ]),
    ];
    
    let dashboard_widget = Paragraph::new(content)
        .wrap(ratatui::widgets::Wrap { trim: true });
    
    frame.render_widget(dashboard_widget, inner);
}

fn render_message(frame: &mut Frame, app: &AppState, area: Rect) {
    if !app.message.is_empty() {
        let message = Paragraph::new(app.message.as_str())
            .style(Style::default().fg(Color::Yellow))
            .alignment(Alignment::Center);
        
        frame.render_widget(message, area);
    }
}

fn render_help(frame: &mut Frame, app: &AppState, area: Rect) {
    let help_text = match app.focus_area {
        FocusArea::Keys => "Keys: ↑/↓=navigate | Tab=next panel | Shows configured SSH keys",
        FocusArea::Groups => "Groups: ↑/↓=navigate | Tab=next panel | Shows host groups",
        FocusArea::Hosts => "Hosts: ↑/↓=navigate | Tab=next panel | Enter=connect (demo) | Shows hosts in selected group",
    };
    
    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    
    frame.render_widget(help, area);
}
