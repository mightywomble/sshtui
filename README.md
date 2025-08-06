# SSH TUI Manager 🦀

A powerful terminal-based SSH connection manager written in Rust, featuring **raw SSH terminal functionality directly within the UI panel** - no mode switching required!

## 🚀 **Key Features**

- **Unified Experience**: Raw terminal works directly in-panel without mode switching
- **Full TUI App Support**: vim, nano, emacs, htop, tmux work perfectly in the terminal panel
- **Always-Visible Sidebar**: Keep your connection list accessible while working
- **Modern Interface**: Mouse and keyboard navigation with colorful, intuitive UI
- **Complete SSH Management**: Add, edit, delete SSH keys, groups, and hosts

## ⚡ **What Works**

- **vim/nano/emacs** work perfectly within the terminal panel
- **htop/top/iotop** display correctly within the panel bounds
- **tmux/screen** sessions run seamlessly in the panel
- **Interactive shells** (Python REPL, database shells) work flawlessly
- **Sidebar stays visible** while using any TUI application
- **Mouse support** for clicking on items, buttons, and scrolling
- **Keyboard shortcuts** for efficient navigation and management

## 🎯 **Technical Achievement**

This implementation showcases:
1. **ANSI escape sequence parsing** using the `vte` crate
2. **Styled terminal content rendering** within ratatui widget bounds
3. **Coordination between TUI framework and raw terminal data**
4. **Precise cursor positioning** and screen clearing within panels
5. **Terminal resizing handling** for both the UI and SSH PTY
6. **Async SSH connections** with proper PTY management

## ✅ **Current Status**

**Fully functional SSH TUI Manager** with:
- ✅ Complete UI layout with sidebar panels
- ✅ Configuration file loading and saving (SSH keys, groups, hosts)
- ✅ Panel navigation and focus management
- ✅ Raw terminal panel with VTE parsing
- ✅ Full SSH connection functionality using portable-pty
- ✅ Add/Edit/Delete operations for keys, groups, and hosts
- ✅ Mouse support for all UI interactions
- ✅ Keyboard shortcuts and navigation
- ✅ Modal dialogs with form handling
- ✅ SSH key selector with dropdown interface
- ✅ Colorful dashboard with live statistics

## 📋 **Requirements**

- Rust 1.70+
- A terminal with Unicode and color support

## 🚦 **Quick Start**

```bash
# Clone and build
git clone <this-repo>
cd sshtuirust
cargo build --release

# Run the demo
cargo run
```

## 🎮 **Controls**

### Keyboard Navigation
- **TAB / Shift+TAB**: Navigate between panels and buttons
- **↑/↓ Arrow Keys**: Navigate within panels and forms
- **Enter**: Connect to selected host or submit forms
- **ESC**: Close modals and cancel operations

### Management Operations
- **Ctrl+N**: Add new (Key/Group/Host depending on focused panel)
- **Ctrl+E**: Edit selected item
- **Ctrl+D**: Delete selected item
- **Ctrl+H**: Show help popup
- **Ctrl+Q**: Quit application or disconnect SSH

### SSH Terminal Controls
- **Ctrl+C**: Send interrupt to SSH session
- **All other keys**: Sent directly to SSH terminal

### Mouse Support
- **Left Click**: Select items, focus panels, click buttons
- **Double Click**: Connect to host (in hosts panel)
- **Scroll Wheel**: Scroll through lists
- **Click outside modal**: Close modal dialogs

## 🏠️ **Architecture**

### Core Components

1. **`config.rs`** - Configuration management (JSON-based with auto-save)
2. **`main.rs`** - Application entry point and main event loop
3. **`ui.rs`** - Main UI rendering and layout management
4. **`ssh.rs`** - SSH connection handling with portable-pty
5. **`terminal_panel.rs`** - Raw terminal panel with VTE parsing
6. **`modal.rs`** - Modal dialogs for forms and user input
7. **`dashboard.rs`** - Welcome screen and statistics display

### Key Technical Elements

- **ratatui** for TUI framework and widget rendering
- **crossterm** for terminal control and mouse/keyboard events
- **VTE parser** for ANSI escape sequence handling
- **portable-pty** for proper SSH PTY management
- **Tokio** for async runtime and SSH connections
- **Serde** for JSON configuration serialization

## 🔬 **The Raw Terminal Panel Concept**

The `RawTerminalPanel` struct (in `terminal_panel.rs`) demonstrates how to:

```rust
impl RawTerminalPanel {
    pub fn write_ssh_data(&mut self, data: &[u8]) {
        // Feed raw SSH data directly to VTE parser
        for &byte in data {
            self.parser.advance(self, byte);
        }
    }
    
    pub fn render(&self, frame: &mut Frame) {
        // Render styled terminal content within ratatui bounds
        // Each character preserves its color, style, and position
    }
}
```

## 🎨 **Visual Experience**

The application features:
- **Colorful dashboard** with live statistics and inspirational quotes
- **Focus highlighting** with yellow borders and clear visual feedback
- **Context-sensitive help** displayed at the bottom
- **Status messages** for user feedback and operation confirmation
- **Smooth mouse interaction** with click feedback
- **Responsive layout** that adapts to terminal size changes

## 🔧 **Configuration**

Configuration is stored in `~/.sshtui.json` and auto-saved when modified:

```json
{
  "groups": [
    {
      "name": "Production",
      "color": "red",
      "hosts": [
        {
          "name": "Web Server",
          "host": "web.example.com",
          "user": "admin",
          "port": 22,
          "key_path": "/home/user/.ssh/id_rsa"
        }
      ]
    },
    {
      "name": "Development",
      "color": "green",
      "hosts": [
        {
          "name": "Dev Server",
          "host": "dev.example.com",
          "user": "developer",
          "port": 2222,
          "key_path": "/home/user/.ssh/dev_key"
        }
      ]
    }
  ],
  "keys": [
    {
      "name": "Default Key",
      "path": "/home/user/.ssh/id_rsa",
      "is_default": true
    },
    {
      "name": "Development Key",
      "path": "/home/user/.ssh/dev_key",
      "is_default": false
    }
  ]
}
```

### Configuration Features
- **Automatic saving** - Changes are persisted immediately
- **Color-coded groups** - Organize hosts with visual distinction
- **SSH key management** - Centralized key storage with dropdown selection
- **Special "All" group** - Automatically shows hosts from all groups

## 🧪 **Implementation Highlights**

This implementation demonstrates that **Rust's lower-level terminal control** combined with **sophisticated parsing libraries** can achieve seamless SSH terminal integration within TUI panels.

**Key Technical Insights:**
- **VTE parser** processes SSH escape sequences while **ratatui handles the overall layout**
- **portable-pty** provides proper PTY management for stable SSH connections
- **Async event handling** allows smooth UI updates while maintaining SSH session responsiveness
- **Smart focus management** enables intuitive navigation between sidebar and terminal

## 🚀 **Future Enhancements**

Possible improvements and additions:
1. **Session management** - Save and restore SSH sessions
2. **Connection profiles** - Quick connect with predefined settings
3. **File transfer integration** - SCP/SFTP support within the UI
4. **Connection health monitoring** - Network latency and status indicators
5. **Scripting support** - Automated command execution
6. **Theme customization** - User-defined color schemes
7. **Connection logging** - Session history and command logging

## 📈 **Performance Benefits**

Rust's advantages for this use case:
- **Zero garbage collection** pauses during intensive terminal operations
- **Memory-safe direct manipulation** of terminal buffers
- **Efficient async I/O** with Tokio
- **Compile-time optimization** of terminal rendering paths

---

**This project demonstrates the feasibility of raw SSH terminal functionality within TUI panels - a significant improvement over mode-based approaches!** 🎉
