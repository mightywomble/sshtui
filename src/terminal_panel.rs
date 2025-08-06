use ratatui::style::Color;
use ratatui::prelude::*;
use std::collections::VecDeque;
use std::io::{stdout, Write};
use tokio::sync::mpsc;
use vte::{Params, Parser, Perform};

/// A terminal panel that can display raw SSH output within specific UI bounds
/// while allowing the TUI framework to control the rest of the screen
pub struct RawTerminalPanel {
    /// Panel bounds within the overall terminal
    bounds: Rect,
    /// Current cursor position within the panel (relative to panel origin)
    cursor_x: u16,
    cursor_y: u16,
    /// Terminal content buffer - each line is a vector of styled characters
    lines: Vec<Vec<StyledChar>>,
    /// VTE parser for handling ANSI escape sequences
    parser: Parser,
    /// Current text style
    current_style: Style,
    /// Whether the panel is currently focused/active
    is_active: bool,
    /// Buffer for accumulating data before processing
    input_buffer: Vec<u8>,
}

#[derive(Clone, Debug)]
struct StyledChar {
    ch: char,
    style: Style,
}

impl Default for StyledChar {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: Style::default(),
        }
    }
}

impl RawTerminalPanel {
    pub fn new(bounds: Rect) -> Self {
        let height = bounds.height as usize;
        let width = bounds.width as usize;
        
        // Initialize with empty lines
        let mut lines = Vec::with_capacity(height);
        for _ in 0..height {
            let mut line = Vec::with_capacity(width);
            for _ in 0..width {
                line.push(StyledChar::default());
            }
            lines.push(line);
        }

        Self {
            bounds,
            cursor_x: 0,
            cursor_y: 0,
            lines,
            parser: Parser::new(),
            current_style: Style::default(),
            is_active: false,
            input_buffer: Vec::new(),
        }
    }

    pub fn set_bounds(&mut self, bounds: Rect) {
        if self.bounds != bounds {
            self.bounds = bounds;
            self.resize_buffer();
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }

    /// Resize the internal buffer to match new bounds
    fn resize_buffer(&mut self) {
        let new_height = self.bounds.height as usize;
        let new_width = self.bounds.width as usize;

        // Preserve existing content where possible
        let mut new_lines = Vec::with_capacity(new_height);
        
        for y in 0..new_height {
            let mut new_line = Vec::with_capacity(new_width);
            
            for x in 0..new_width {
                if y < self.lines.len() && x < self.lines[y].len() {
                    new_line.push(self.lines[y][x].clone());
                } else {
                    new_line.push(StyledChar::default());
                }
            }
            new_lines.push(new_line);
        }

        self.lines = new_lines;

        // Adjust cursor position if needed
        if self.cursor_x >= self.bounds.width {
            self.cursor_x = self.bounds.width.saturating_sub(1);
        }
        if self.cursor_y >= self.bounds.height {
            self.cursor_y = self.bounds.height.saturating_sub(1);
        }
    }

    /// Process SSH output data - this is where the raw terminal magic happens
    pub fn write_ssh_data(&mut self, data: &[u8]) {
        // Store data temporarily and process it with VTE parser
        self.input_buffer.extend_from_slice(data);
        
        // Process all buffered data
        let buffer_copy = self.input_buffer.clone();
        self.input_buffer.clear();
        
        // Process each byte through VTE parser
        for byte in buffer_copy {
            // We need to handle the borrowing issue by separating parser from self
            let mut temp_parser = std::mem::replace(&mut self.parser, Parser::new());
            temp_parser.advance(self, byte);
            self.parser = temp_parser;
        }
    }
    
    fn write_char_at_cursor(&mut self, ch: char) {
        let inner_width = (self.bounds.width.saturating_sub(2)) as usize;
        let inner_height = (self.bounds.height.saturating_sub(2)) as usize;
        
        if (self.cursor_y as usize) < self.lines.len() && (self.cursor_x as usize) < inner_width {
            let line = &mut self.lines[self.cursor_y as usize];
            if (self.cursor_x as usize) < line.len() {
                line[self.cursor_x as usize] = StyledChar {
                    ch,
                    style: self.current_style,
                };
            }
        }

        self.cursor_x += 1;
        if self.cursor_x >= inner_width as u16 {
            // Line wrap
            self.cursor_x = 0;
            self.cursor_y += 1;
            if self.cursor_y >= inner_height as u16 {
                self.scroll_up();
                self.cursor_y = inner_height.saturating_sub(1) as u16;
            }
        }
    }

    /// Render the terminal panel content to the screen
    /// This integrates with the TUI framework but writes raw content to our panel area
    pub fn render(&self, frame: &mut Frame) {
        // Create block for the terminal panel
        let block = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .title("SSH Terminal")
            .border_style(if self.is_active {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            });

        // Calculate inner area for terminal content first
        let inner = block.inner(self.bounds);
        
        // Render block
        frame.render_widget(block, self.bounds);
        
        // Render terminal content line by line
        for (y, line) in self.lines.iter().enumerate() {
            if y >= inner.height as usize {
                break;
            }

            let mut spans = Vec::new();
            let mut current_span_text = String::new();
            let mut current_span_style = Style::default();

            for (x, styled_char) in line.iter().enumerate() {
                if x >= inner.width as usize {
                    break;
                }

                // If style changes, flush current span and start new one
                if styled_char.style != current_span_style && !current_span_text.is_empty() {
                    spans.push(Span::styled(current_span_text, current_span_style));
                    current_span_text = String::new();
                }

                current_span_style = styled_char.style;
                current_span_text.push(styled_char.ch);
            }

            // Flush remaining text
            if !current_span_text.is_empty() {
                spans.push(Span::styled(current_span_text, current_span_style));
            }

            // Render this line
            let line_widget = ratatui::widgets::Paragraph::new(Line::from(spans));
            let line_area = Rect {
                x: inner.x,
                y: inner.y + y as u16,
                width: inner.width,
                height: 1,
            };
            
            frame.render_widget(line_widget, line_area);
        }

        // Render cursor if active
        if self.is_active && self.cursor_y < inner.height && self.cursor_x < inner.width {
            let cursor_area = Rect {
                x: inner.x + self.cursor_x,
                y: inner.y + self.cursor_y,
                width: 1,
                height: 1,
            };
            
            let cursor_widget = ratatui::widgets::Block::default()
                .style(Style::default().bg(Color::White).fg(Color::Black));
            
            frame.render_widget(cursor_widget, cursor_area);
        }
    }

    /// Get the current cursor position for PTY sizing
    pub fn get_size(&self) -> (u16, u16) {
        let inner_width = self.bounds.width.saturating_sub(2); // Account for borders
        let inner_height = self.bounds.height.saturating_sub(2);
        (inner_width, inner_height)
    }

    /// Clear the terminal content
    pub fn clear(&mut self) {
        for line in &mut self.lines {
            for styled_char in line {
                *styled_char = StyledChar::default();
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    /// Scroll the terminal content up by one line
    fn scroll_up(&mut self) {
        // Move all lines up
        for i in 1..self.lines.len() {
            self.lines[i - 1] = self.lines[i].clone();
        }
        
        // Clear the last line
        if let Some(last_line) = self.lines.last_mut() {
            for styled_char in last_line {
                *styled_char = StyledChar::default();
            }
        }
    }

    /// Write a character at the current cursor position
    fn write_char(&mut self, ch: char) {
        let inner_width = self.bounds.width.saturating_sub(2) as usize;
        let inner_height = self.bounds.height.saturating_sub(2) as usize;

        match ch {
            '\n' => {
                // Newline - move to next line
                self.cursor_x = 0;
                self.cursor_y += 1;
                if self.cursor_y >= inner_height as u16 {
                    self.scroll_up();
                    self.cursor_y = inner_height.saturating_sub(1) as u16;
                }
            },
            '\r' => {
                // Carriage return - move to start of line
                self.cursor_x = 0;
            },
            '\t' => {
                // Tab - move to next tab stop (every 8 characters)
                let next_tab = ((self.cursor_x / 8) + 1) * 8;
                self.cursor_x = next_tab.min(inner_width.saturating_sub(1) as u16);
            },
            ch if ch.is_control() => {
                // Skip other control characters
            },
            _ => {
                // Regular character - write it
                if (self.cursor_y as usize) < self.lines.len() && (self.cursor_x as usize) < inner_width {
                    let line = &mut self.lines[self.cursor_y as usize];
                    if (self.cursor_x as usize) < line.len() {
                        line[self.cursor_x as usize] = StyledChar {
                            ch,
                            style: self.current_style,
                        };
                    }
                }

                self.cursor_x += 1;
                if self.cursor_x >= inner_width as u16 {
                    // Line wrap
                    self.cursor_x = 0;
                    self.cursor_y += 1;
                    if self.cursor_y >= inner_height as u16 {
                        self.scroll_up();
                        self.cursor_y = inner_height.saturating_sub(1) as u16;
                    }
                }
            }
        }
    }
}

/// Implement the VTE Perform trait to handle ANSI escape sequences
impl Perform for RawTerminalPanel {
    fn print(&mut self, c: char) {
        self.write_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.write_char('\n'),
            b'\r' => self.write_char('\r'),
            b'\t' => self.write_char('\t'),
            0x08 => {
                // Backspace
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                }
            },
            _ => {} // Ignore other control characters for now
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _c: char) {
        // Handle DCS sequences if needed
    }

    fn put(&mut self, _byte: u8) {
        // Handle DCS data if needed
    }

    fn unhook(&mut self) {
        // End DCS sequence
    }

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        // Handle OSC sequences (like setting window title) if needed
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, c: char) {
        match c {
            'A' => {
                // Cursor up
                let n = params.iter().next().unwrap_or(&[1])[0] as u16;
                self.cursor_y = self.cursor_y.saturating_sub(n);
            },
            'B' => {
                // Cursor down
                let n = params.iter().next().unwrap_or(&[1])[0] as u16;
                self.cursor_y = (self.cursor_y + n).min(self.bounds.height.saturating_sub(3));
            },
            'C' => {
                // Cursor forward
                let n = params.iter().next().unwrap_or(&[1])[0] as u16;
                self.cursor_x = (self.cursor_x + n).min(self.bounds.width.saturating_sub(3));
            },
            'D' => {
                // Cursor back
                let n = params.iter().next().unwrap_or(&[1])[0] as u16;
                self.cursor_x = self.cursor_x.saturating_sub(n);
            },
            'H' | 'f' => {
                // Cursor position
                let row = params.iter().next().unwrap_or(&[1])[0] as u16;
                let col = params.iter().nth(1).unwrap_or(&[1])[0] as u16;
                self.cursor_y = (row.saturating_sub(1)).min(self.bounds.height.saturating_sub(3));
                self.cursor_x = (col.saturating_sub(1)).min(self.bounds.width.saturating_sub(3));
            },
            'J' => {
                // Clear screen
                let n = params.iter().next().unwrap_or(&[0])[0];
                match n {
                    0 => {
                        // Clear from cursor to end of screen
                        self.clear_from_cursor();
                    },
                    1 => {
                        // Clear from start of screen to cursor
                        self.clear_to_cursor();
                    },
                    2 => {
                        // Clear entire screen
                        self.clear();
                    },
                    _ => {}
                }
            },
            'K' => {
                // Clear line
                let n = params.iter().next().unwrap_or(&[0])[0];
                match n {
                    0 => {
                        // Clear from cursor to end of line
                        if (self.cursor_y as usize) < self.lines.len() {
                            let line = &mut self.lines[self.cursor_y as usize];
                            for x in (self.cursor_x as usize)..line.len() {
                                line[x] = StyledChar::default();
                            }
                        }
                    },
                    1 => {
                        // Clear from start of line to cursor
                        if (self.cursor_y as usize) < self.lines.len() {
                            let line = &mut self.lines[self.cursor_y as usize];
                            for x in 0..=(self.cursor_x as usize).min(line.len().saturating_sub(1)) {
                                line[x] = StyledChar::default();
                            }
                        }
                    },
                    2 => {
                        // Clear entire line
                        if (self.cursor_y as usize) < self.lines.len() {
                            let line = &mut self.lines[self.cursor_y as usize];
                            for styled_char in line {
                                *styled_char = StyledChar::default();
                            }
                        }
                    },
                    _ => {}
                }
            },
            'm' => {
                // Set graphics rendition (colors, bold, etc.)
                self.handle_sgr(params);
            },
            _ => {
                // Ignore other CSI sequences for now
            }
        }
    }

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {
        // Handle ESC sequences if needed
    }
}

impl RawTerminalPanel {
    fn clear_from_cursor(&mut self) {
        // Clear from cursor to end of current line
        if (self.cursor_y as usize) < self.lines.len() {
            let line = &mut self.lines[self.cursor_y as usize];
            for x in (self.cursor_x as usize)..line.len() {
                line[x] = StyledChar::default();
            }
        }

        // Clear all lines below current line
        for y in (self.cursor_y as usize + 1)..self.lines.len() {
            for styled_char in &mut self.lines[y] {
                *styled_char = StyledChar::default();
            }
        }
    }

    fn clear_to_cursor(&mut self) {
        // Clear all lines above current line
        for y in 0..(self.cursor_y as usize) {
            if y < self.lines.len() {
                for styled_char in &mut self.lines[y] {
                    *styled_char = StyledChar::default();
                }
            }
        }

        // Clear from start of current line to cursor
        if (self.cursor_y as usize) < self.lines.len() {
            let line = &mut self.lines[self.cursor_y as usize];
            for x in 0..=(self.cursor_x as usize).min(line.len().saturating_sub(1)) {
                line[x] = StyledChar::default();
            }
        }
    }

    fn handle_sgr(&mut self, params: &Params) {
        for param in params.iter() {
            let n = param[0];
            match n {
                0 => {
                    // Reset all attributes
                    self.current_style = Style::default();
                },
                1 => {
                    // Bold
                    self.current_style = self.current_style.add_modifier(Modifier::BOLD);
                },
                4 => {
                    // Underline
                    self.current_style = self.current_style.add_modifier(Modifier::UNDERLINED);
                },
                30..=37 => {
                    // Foreground colors
                    let color = match n {
                        30 => Color::Black,
                        31 => Color::Red,
                        32 => Color::Green,
                        33 => Color::Yellow,
                        34 => Color::Blue,
                        35 => Color::Magenta,
                        36 => Color::Cyan,
                        37 => Color::White,
                        _ => Color::White,
                    };
                    self.current_style = self.current_style.fg(color);
                },
                40..=47 => {
                    // Background colors
                    let color = match n {
                        40 => Color::Black,
                        41 => Color::Red,
                        42 => Color::Green,
                        43 => Color::Yellow,
                        44 => Color::Blue,
                        45 => Color::Magenta,
                        46 => Color::Cyan,
                        47 => Color::White,
                        _ => Color::Black,
                    };
                    self.current_style = self.current_style.bg(color);
                },
                90..=97 => {
                    // Bright foreground colors
                    let color = match n {
                        90 => Color::DarkGray,
                        91 => Color::LightRed,
                        92 => Color::LightGreen,
                        93 => Color::LightYellow,
                        94 => Color::LightBlue,
                        95 => Color::LightMagenta,
                        96 => Color::LightCyan,
                        97 => Color::White,
                        _ => Color::White,
                    };
                    self.current_style = self.current_style.fg(color);
                },
                _ => {} // Ignore unknown SGR parameters
            }
        }
    }
}
