use chrono::Local;
use ratatui::prelude::*;

// Simple demo function
pub fn render_simple_dashboard(_width: u16, _height: u16) -> Text<'static> {
    let mut lines = Vec::new();
    
    lines.push(Line::from(vec![
        Span::styled(
            "🦀 Rust SSH TUI Demo",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        )
    ]));
    
    lines.push(Line::from("Raw terminal in panel concept demonstrated!"));
    
    Text::from(lines)
}

// Original function with conditional compilation
pub fn render_dashboard(app: &crate::AppState, width: u16, height: u16) -> Text {
    let mut lines = Vec::new();
    
    // Welcome message
    lines.push(Line::from(vec![
        Span::styled(
            "🚀 Welcome to SSH TUI Manager (Rust)!",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        )
    ]));
    lines.push(Line::from(""));
    
    // Statistics section
    lines.push(Line::from(vec![
        Span::styled(
            "📊 CURRENT STATISTICS",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        )
    ]));
    
    let total_keys = app.config.keys.len();
    let total_groups = app.config.groups.len().saturating_sub(1); // Subtract "All" group
    let total_hosts: usize = app.config.groups.iter().skip(1).map(|g| g.hosts.len()).sum();
    
    lines.push(Line::from(vec![
        Span::styled("🔑 SSH Keys: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", total_keys),
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        )
    ]));
    
    lines.push(Line::from(vec![
        Span::styled("📁 Groups: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", total_groups),
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        )
    ]));
    
    lines.push(Line::from(vec![
        Span::styled("🖥️  Total Hosts: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{}", total_hosts),
            Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD)
        )
    ]));
    lines.push(Line::from(""));
    
    // Action guidance
    if total_hosts > 0 {
        lines.push(Line::from(vec![
            Span::styled(
                "⚡ QUICK ACTIONS",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            )
        ]));
        
        let actions = [
            "• Select a host and press ENTER to connect",
            "• Navigate with TAB or arrow keys",
            "• Use [+/E/D] buttons to manage items",
            "• All keyboard input goes directly to SSH when connected",
        ];
        
        for action in &actions {
            lines.push(Line::from(vec![
                Span::styled(*action, Style::default().fg(Color::Gray))
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled(
                "🎯 GET STARTED",
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            )
        ]));
        
        let steps = [
            "1. Add SSH keys in the top-left panel",
            "2. Create groups in the middle-left panel", 
            "3. Add hosts to groups in the bottom-left panel",
            "4. Connect and enjoy raw terminal experience!",
        ];
        
        for step in &steps {
            lines.push(Line::from(vec![
                Span::styled(*step, Style::default().fg(Color::Gray))
            ]));
        }
    }
    lines.push(Line::from(""));
    
    // Current focus info
    lines.push(Line::from(vec![
        Span::styled(
            "🎯 CURRENT FOCUS",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        )
    ]));
    
    let focus_area = match app.focus_area {
        crate::FocusArea::Keys => "SSH Keys",
        crate::FocusArea::Groups => "Groups",
        crate::FocusArea::Hosts => "Hosts",
    };
    
    let focus_sub_area = match app.focus_sub_area {
        crate::FocusSubArea::Items => "Items",
        crate::FocusSubArea::AddButton => "Add Button",
        crate::FocusSubArea::EditButton => "Edit Button", 
        crate::FocusSubArea::DeleteButton => "Delete Button",
    };
    
    lines.push(Line::from(vec![
        Span::styled(
            format!("Panel: {} | Sub-focus: {}", focus_area, focus_sub_area),
            Style::default().fg(Color::Gray)
        )
    ]));
    lines.push(Line::from(""));
    
    // Inspirational quote
    let quotes = [
        "\"Secure connections, infinite possibilities.\"",
        "\"SSH: Your gateway to remote worlds.\"", 
        "\"Connect securely, work efficiently.\"",
        "\"Remote access made simple and secure.\"",
        "\"One key, many doors.\"",
        "\"Raw terminal power in the palm of your hand.\"",
    ];
    
    let quote_index = (Local::now().timestamp() as usize) % quotes.len();
    lines.push(Line::from(vec![
        Span::styled(
            quotes[quote_index],
            Style::default().fg(Color::Magenta).add_modifier(Modifier::ITALIC)
        )
    ]));
    lines.push(Line::from(""));
    
    // Current time
    let current_time = Local::now().format("%a %b %d, %Y %H:%M:%S").to_string();
    lines.push(Line::from(vec![
        Span::styled(
            format!("🕒 {}", current_time),
            Style::default().fg(Color::LightBlue).add_modifier(Modifier::ITALIC)
        )
    ]));
    lines.push(Line::from(""));
    
    // Rust advantage note
    lines.push(Line::from(vec![
        Span::styled(
            "⚡ This Rust version features raw SSH terminal in-panel!",
            Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD)
        )
    ]));
    lines.push(Line::from(vec![
        Span::styled(
            "Perfect for vim, htop, and other TUI apps without mode switching!",
            Style::default().fg(Color::LightGreen)
        )
    ]));
    
    // Truncate if needed to fit in panel
    if lines.len() > height as usize {
        lines.truncate(height as usize - 1);
        lines.push(Line::from(vec![
            Span::styled("... (content truncated)", Style::default().fg(Color::DarkGray))
        ]));
    }
    
    Text::from(lines)
}
