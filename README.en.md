# SerialSSHTerm

Language: [Français](README.md) | **English**

A professional serial and SSH terminal written in **Rust** with **GTK4** and **Libadwaita**.

## 🎯 Overview

SerialSSHTerm is a unified terminal app to communicate with devices over **serial** or **SSH** in a modern desktop interface.

Main features:

- 🔌 Configurable **serial connection** (baud rate, data bits, parity, stop bits, flow control)
- 🔐 **SSH connection** with password or private key auth and TOFU host key verification
- 🖥️ Full **ANSI terminal emulation** (colors, SGR, escape sequences)
- 📝 Real-time output with configurable scrollback
- 💾 Save logs to text files
- 🎨 Themes (Light, Dark, Hacker)
- ⚙️ Persistent JSON settings
- 🔐 SSH secrets stored in system keyring (Linux Secret Service / Windows Credential Manager)
- 🔔 Non-blocking Adwaita toast notifications
- 🧮 Built-in tools: calculator and DEC/HEX/BIN converter

## 🚀 Installation

### Option 1: Debian package (.deb)

```bash
./install-deps.sh
./build-deb.sh
sudo dpkg -i ../serial-ssh-term_1.0.0*.deb
```

### Option 1b: Windows build (.exe)

On Windows 11 (PowerShell):

```powershell
# 1) Install dependencies (Rust + MSYS2 + GTK4/libadwaita)
powershell -ExecutionPolicy Bypass -File .\install-deps-windows.ps1

# 2) Build exe + distributable ZIP
powershell -ExecutionPolicy Bypass -File .\build-exe.ps1 -IncludeGtkRuntime

# 3) Install Inno Setup (once)
winget install JRSoftware.InnoSetup

# 4) Build Windows installer (.exe)
powershell -ExecutionPolicy Bypass -File .\build-installer.ps1 -IncludeGtkRuntime
```

Generated artifacts:

- `dist/windows/SerialSSHTerm/serial-ssh-term.exe`
- `dist/windows/serial-ssh-term-win64-release.zip`
- `dist/windows/installer/serial-ssh-term-setup-win64-v<version>.exe`

### Option 2: Build from source

Requirements:

- Rust 1.75+
- GTK 4.14+ dev packages
- OpenSSL dev package (required by `russh`)

Build and run:

```bash
cargo build --release
./target/release/serial-ssh-term
```

## 📖 Usage

### Serial connection

1. Select **🔌 Serial** tab
2. Choose the port
3. Configure serial parameters
4. Click **Connect**
5. Type commands and press Enter

### SSH connection

1. Select **🔐 SSH** tab
2. Enter host, port, username
3. Choose auth method (password or key)
4. Toggle **Remember secrets** based on your policy
5. Click **Connect**
6. On first host contact, TOFU dialog asks for host key confirmation

## ⚙️ Configuration

Settings file:

```text
~/.config/serial-ssh-term/settings.json
```

It includes serial/SSH preferences, UI settings, and line ending options.

Secrets (SSH password / key passphrase) are not stored in `settings.json`; they are stored in the OS keyring. `remember_secrets` controls this behavior.

## 🛠️ Architecture

Three-layer architecture:

- UI layer (`src/ui`) on GTK thread
- Async bridge (`async_channel` + GLib timer)
- Core layer (`src/core`) running I/O via Tokio actor

`spawn_connection_actor` keeps all connection I/O off the UI thread.

## 📦 Packaging scripts

- Debian: `build-deb.sh`, `install-deps.sh`
- Windows: `build-exe.ps1`, `build-installer.ps1`, `install-deps-windows.ps1`

See also: `PACKAGING.md`.

## 📄 License

MIT
