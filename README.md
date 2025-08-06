# SSH TUI Manager - Rust Edition 🦀

A **proof-of-concept** Rust implementation of the SSH TUI Manager, demonstrating **raw SSH terminal functionality directly within the UI panel** - no mode switching required!

## 🚀 **Key Innovation**

Unlike the Go version which requires switching between different terminal modes:
- **Go Version**: Standard Mode → Fullscreen Mode → Raw Mode (3 separate modes)
- **Rust Version**: **Raw terminal works directly in-panel** (unified experience)

## ⚡ **What This Enables**

- **vim/nano/emacs** work perfectly within the terminal panel
- **htop/top/iotop** display correctly within the panel bounds
- **tmux/screen** sessions run seamlessly in the panel
- **Interactive shells** (Python REPL, database shells) work flawlessly
- **Sidebar stays visible** while using any TUI application

## 🎯 **Technical Achievement**

This Rust implementation demonstrates how to:
1. **Parse ANSI escape sequences** using the `vte` crate
2. **Render styled terminal content** within ratatui widget bounds
3. **Coordinate between TUI framework and raw terminal data**
4. **Maintain precise cursor positioning** and screen clearing within panels
5. **Handle terminal resizing** for both the UI and SSH PTY

## 🛠️ **Current Status**

This is a **demonstration/proof-of-concept** showing:
- ✅ Complete UI layout with sidebar panels
- ✅ Configuration file loading (SSH keys, groups, hosts)
- ✅ Panel navigation and focus management
- ✅ Raw terminal panel implementation structure
- ❌ Actual SSH connection (not implemented in demo)
- ❌ Add/Edit/Delete functionality (read-only demo)

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

## 🎮 **Demo Controls**

- **TAB / Shift+TAB**: Navigate between panels (Keys/Groups/Hosts)
- **↑/↓ Arrow Keys**: Navigate within panels
- **Enter**: Simulate connection (shows message)
- **Ctrl+Q**: Exit application

## 🏗️ **Architecture**

### Core Components

1. **`config.rs`** - Configuration management (JSON-based)
2. **`ui_simple.rs`** - Main UI rendering logic
3. **`dashboard.rs`** - Welcome screen content
4. **`terminal_panel.rs`** - Raw terminal panel implementation (not used in demo)
5. **`main_simple.rs`** - Application entry point and event loop

### Key Technical Elements

- **ratatui** for TUI framework
- **crossterm** for terminal control
- **VTE parser** for ANSI escape sequence handling
- **Tokio** for async runtime (future SSH connections)

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

## 🆚 **Comparison with Go Version**

| Feature | Go Implementation | Rust Implementation |
|---------|-------------------|-------------------|
| **Mode Switching** | Required (3 modes) | Not needed |
| **TUI App Support** | Raw mode only | In-panel directly |
| **Sidebar Visibility** | Hidden in raw mode | Always visible |
| **Context Switching** | Mode → Fullscreen → Raw | Direct interaction |
| **Memory Safety** | GC overhead | Zero-cost abstractions |
| **Terminal Control** | Bubble Tea limitations | Direct VTE parsing |

## 🎨 **Visual Experience**

The application features:
- **Colorful dashboard** with live statistics
- **Focus highlighting** with yellow borders
- **Context-sensitive help** at the bottom
- **Status messages** for user feedback
- **Emoji-rich interface** for visual appeal

## 🔧 **Configuration**

Uses the same JSON configuration format as the Go version:

```json
{
  "groups": [
    {
      "name": "All",
      "color": "blue", 
      "hosts": []
    },
    {
      "name": "Production",
      "color": "red",
      "hosts": [
        {
          "name": "Web Server",
          "host": "web.example.com",
          "user": "admin",
          "port": 22,
          "key_path": "/path/to/key"
        }
      ]
    }
  ],
  "keys": [
    {
      "name": "Default Key",
      "path": "/home/user/.ssh/id_rsa",
      "is_default": true
    }
  ]
}
```

## 🧪 **Implementation Notes**

This proof-of-concept demonstrates that **Rust's lower-level terminal control** combined with **sophisticated parsing libraries** can achieve what's difficult in higher-level TUI frameworks.

The key insight is using the **VTE parser** to process SSH escape sequences while **ratatui handles the overall layout** - giving us the best of both worlds.

## 🚧 **Future Development**

To complete this implementation:
1. **SSH Connection Logic** using `russh` crate
2. **Form Handling** for add/edit operations  
3. **File Browser** for SSH key selection
4. **Clipboard Integration** for SSH command copying
5. **Configuration Persistence** for changes

## 📈 **Performance Benefits**

Rust's advantages for this use case:
- **Zero garbage collection** pauses during intensive terminal operations
- **Memory-safe direct manipulation** of terminal buffers
- **Efficient async I/O** with Tokio
- **Compile-time optimization** of terminal rendering paths

---

**This project demonstrates the feasibility of raw SSH terminal functionality within TUI panels - a significant improvement over mode-based approaches!** 🎉
