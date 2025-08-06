# SSH TUI Testing Instructions

## What's New

### 1. SSH Key Selector Feature
- When adding/editing hosts, you can now choose between:
  - **Key Selector Mode**: Dropdown list of configured SSH keys
  - **Manual Mode**: Direct path input for SSH keys

### 2. Fixed SSH Connection Issues
- Improved connection stability with ServerAlive options
- Better error handling and logging
- Fixed process management to prevent premature disconnections

### 3. Improved UI
- White background fields now have black text for better readability
- Visual indicators show which mode you're in

## How to Test

### 1. Launch the Application
```bash
./target/release/sshtuirust
```

### 2. Test SSH Key Selector
1. Press `Tab` to focus on the Hosts area
2. Press `Ctrl+N` to add a new host
3. Navigate to the SSH Key field (5th field)
4. You should see a dropdown showing "▼ default" (since default is the first key)
5. Use `j`/`k` or `↑`/`↓` arrows to select different keys
6. Press `s` to switch to manual path input mode
7. Press `s` again to switch back to selector mode

### 3. Test SSH Connections
1. Try connecting to the localhost entry (select it and press Enter)
2. The connection should stay alive and show the shell prompt
3. Type commands and see the output in the terminal panel

### 4. Test Modal Navigation
1. Try adding keys with `Ctrl+N` when focused on Keys
2. Try adding groups with `Ctrl+N` when focused on Groups  
3. Try editing existing entries with `Ctrl+E`
4. Notice the improved text visibility in modal forms

## Configuration
The app now includes:
- 3 SSH keys: default, server_key, and test_key
- Multiple host configurations for testing
- Color-coded groups (Development=green, Production=red)

## Keyboard Shortcuts
- `Tab` - Navigate between sections
- `Ctrl+N` - Add new item
- `Ctrl+E` - Edit selected item
- `Enter` - Connect to selected host
- `Esc` - Cancel modal/disconnect
- `j`/`k` or `↑`/`↓` - Navigate in key selector
- `s` - Toggle between key selector and manual input

## Expected Behavior
- SSH connections should remain stable
- Modal forms should be clearly readable
- Key selector should show available SSH keys
- Switching between modes should work smoothly
