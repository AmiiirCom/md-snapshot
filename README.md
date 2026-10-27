# 📷 MD Snapshot

[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20Linux%20%7C%20macOS-blue.svg?style=flat-square)](https://github.com/)
[![License](https://img.shields.io/badge/License-MIT-green.svg?style=flat-square)](LICENSE)

**MD Snapshot** is a fast, token-optimized, cross-platform desktop application built with **Rust** and **eframe/egui**. It converts any software project directory into a compact, clean Markdown prompt ready for Large Language Models (**ChatGPT**, **Claude**, **DeepSeek**, **Gemini**).

---

## ✨ Features

- **⚡ Fast & Responsive**: File scanning, binary detection, and formatting are processed on background worker threads with cancellation support. The UI never freezes.
- **📄 Custom Instruction Docs**: Attach external context files (e.g., `skill.md`, `agent.md`, system prompts, coding guidelines) to be injected at the very top of your prompt.
- **🌲 Compact Visual Tree**: Generates an ASCII file tree of included files.
- **🛡️ Safe & Clean**:
  - **No Path Leaks**: All paths are sanitized to relative paths; user home/absolute directories are never exposed.
  - **Adaptive Code Fences**: Dynamically adjusts backtick lengths (` ```` `, ` ````` `) to prevent nested code blocks from breaking Markdown syntax.
  - **Binary Filtering**: Automatic UTF-8 validation and null-byte heuristics to filter out binary/compiled assets.
  - **Symlink Protection**: Does not traverse symlinks to avoid directory loops and out-of-root leaks.
- **🎯 Token-Optimized**:
  - **SHA-256 Deduplication**: Identifies identical files and replaces redundant code with references.
  - **Smart Trimming**: Removes trailing empty lines while strictly preserving internal code indentation and spacing.
  - **No Silent Omissions**: Files skipped due to size limits are explicitly documented in an `Omitted / Skipped Files` section.
- **📊 Real-time Stats**: Live preview, character counter, file metrics, and token estimation (~4 chars/token).
- **🪟 Windows Integration**: Native Windows `.exe` embedding with auto-generated embedded resource icon and no console popups (`#![windows_subsystem = "windows"]`).

---

## 📋 Markdown Output Structure

The output Markdown document follows a structured layout optimized for LLM comprehension:

```markdown
# Project: <Project Name>

## Context & Instructions
### skill.md
````markdown
<Content of attached prompt or role document>
````

## Project Tree
```text
├── src
│   ├── main.rs
│   └── app.rs
└── Cargo.toml
```

## Source Files
### src/main.rs
````rust
<File content with internal formatting preserved>
````

### src/duplicate.rs
*(Identical content to `src/main.rs`)*

## Omitted / Skipped Files
- `assets/heavy_payload.json`: File exceeds size limit (512 KB)
```

---

## 🚀 Download & Installation

### Pre-built Binaries
Download the latest pre-compiled binaries for **Windows**, **Linux**, and **macOS** from the [Releases](https://github.com/) page.

### Building from Source

#### Prerequisites
Install the [Rust toolchain](https://rustup.rs/) (edition 2021 or later).

#### Linux Dependencies (Debian / Ubuntu)
For Linux desktop environments, install the required GTK and graphics development packages:
```bash
sudo apt update
sudo apt install -y pkg-config libglib2.0-dev libgtk-3-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev
```

#### Compilation
Clone the repository and compile the optimized release binary:

```bash
git clone https://github.com/yourusername/md-snapshot.git
cd md-snapshot

# Build for release
cargo build --release
```

The resulting binary will be available in:
- **Windows**: `target/release/md-snapshot.exe` *(embedded with the MDS application icon)*
- **Linux / macOS**: `target/release/md-snapshot`

---

## 🛠️ Configuration & Defaults

| Setting | Default Value | Description |
| :--- | :--- | :--- |
| **Allowed Extensions** | `rs, toml, md, json, yaml, yml, py, js, ts, tsx, jsx, html, css, go, java, c, h, cpp, hpp, sh, sql` | Configurable whitelist for code files. |
| **Ignored Directories** | `.git, target, node_modules, vendor, dist, build, .idea, .vscode, __pycache__, .next` | Skipped during project traversal. |
| **Per-File Size Limit** | `512 KB` | Files larger than this threshold are skipped and logged. |
| **Total Output Limit** | `10 MB` | Hard ceiling for total Markdown output size. |
| **Deduplication** | Enabled | SHA-256 matching avoids repeating duplicate file bodies. |

---

## 🤝 Contributing

Contributions, issues, and feature requests are welcome! Feel free to check the [issues page](https://github.com/yourusername/md-snapshot/issues).

---

## 📄 License

This project is licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.