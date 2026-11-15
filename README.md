# 📦 md-snapshot

> **Fast, token-optimized desktop app to pack codebases and docs into clean LLM context.**  
> Built with high-performance **Rust**, **egui**, and **eframe**.

[![Release](https://img.shields.io/github/v/release/AmiiirCom/md-snapshot?style=flat-square&color=89b4fa)](https://github.com/AmiiirCom/md-snapshot/releases)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-informational?style=flat-square)](https://github.com/AmiiirCom/md-snapshot/releases)

---

## 🌟 Overview

**md-snapshot** scans your software project directory and transforms it into a clean, compact, and structured prompt ready for Large Language Models (**Claude 3.5**, **ChatGPT / GPT-4o**, **DeepSeek**, **Gemini**).

Unlike bloated Electron-based alternatives, **md-snapshot** is a native, ultra-lightweight Rust desktop application that launches instantly, handles massive monorepos with ease, and never freezes your UI.

---

## 🚀 Key Features

### 📑 Multi-Format LLM Exports
Export your codebase in the format that best suits your target AI model:
- **Markdown (`.md`)** *(Default)*: Clean markdown structure with dynamic code fences (` ``` `, ` ```` `, ` ````` `) preventing nested block syntax breakage.
- **XML (`.xml`)**: Structured tags (`<file path="...">`, `<instructions>`, `<file_tree>`) optimized for **Anthropic Claude** and modern coding agents.
- **YAML (`.yaml`)**: Clean hierarchical structure using literal block scalars.

### 🌿 Interactive File & Directory Tree (`Custom` Preset)
- **Tri-State Checkboxes (`☑ / ☐ / ▣`)**: Easily check or uncheck individual files or entire folders. Unchecking a directory completely omits all of its nested files.
- **Hidden & Config Files**: Select sensitive or hidden files like `.env`, `.gitignore`, `.prettierrc`, `Dockerfile`, or folders like `.github` and `.vscode`.
- **Live Search & Filter**: Quickly find files in large repositories with instant substring search.
- **Quick Controls**: 1-click `Select All`, `Deselect All`, `Expand All`, and `Collapse All`.

### ⚡ 1-Click Language & Stack Presets
Switch contexts instantly without manually retyping extensions:
- 🦀 **Rust**: `.rs`, `.toml`, `.md`
- ⚡ **Web / TypeScript**: `.ts`, `.tsx`, `.js`, `.jsx`, `.json`, `.html`, `.css`, `.md`
- 🐍 **Python**: `.py`, `.toml`, `.yaml`, `.yml`, `.sql`, `.md`
- 🐹 **Go**: `.go`, `.mod`, `.sum`, `.md`
- 🌐 **Fullstack**: Combined modern multi-language stack.
- 🛠 **Custom**: Unlocks the full interactive file tree for surgical file selection.

### 🏷️ Smart Tag / Chip Input System
- Visual tags for allowed extensions and ignored folders with 1-click removal (`✕`).
- Quick input box with `+ Add` button or <kbd>Enter</kbd> key.
- Deterministic sorting to eliminate jumping or layout shifts.

### 🔤 Live Token Metrics & Context Indicators
- **Real-Time Token Estimation**: Instant token calculation (~4 chars/token).
- **Context Window Usage**: Live visual gauges showing percentage of context filled for **GPT-4o (128k)** and **Claude 3.5 Sonnet (200k)**.

### 🛡️ Safe, Clean & Token-Optimized
- **SHA-256 Deduplication**: Identifies identical files and replaces duplicate content with concise pointers.
- **Binary & Symlink Protection**: UTF-8 validation and null-byte heuristics prevent binary bloat; protects against circular symlinks.
- **Path Sanitization**: Strips absolute system paths, protecting local usernames and directory structures.
- **Strip Comments (Optional)**: Strip line comments (`//`, `#`) to reduce token consumption by 20–30%.
- **Optional `.gitignore` Support**: Inherit Git ignore rules with a single toggle (OFF by default).
- **Omitted Files Log**: Explicitly reports any skipped or oversized files.

### 💻 Modern Desktop UX
- **Drag & Drop**: Drop project folders or external instruction docs (`skill.md`, `agent.md`) straight into the window.
- **Export Actions**: 1-click **Copy to Clipboard** with visual toast confirmation, or **Save As...** directly to disk.
- **Native Look**: Built-in modern dark theme (Linear/Catppuccin inspired) with smooth high-DPI scaling.

---

## 📥 Download & Installation

Pre-compiled release binaries are automatically built for Windows, Linux, and macOS:

👉 **[Download Latest Release](https://github.com/AmiiirCom/md-snapshot/releases/latest)**

| OS | Format | Architecture |
|---|---|---|
| **Windows** | `.zip` (Standalone `.exe`) | x86_64 |
| **Linux (Ubuntu/Debian)** | `.tar.gz` | x86_64 |
| **macOS** | `.tar.gz` | Apple Silicon (arm64) |

---

## 🛠️ Building from Source

### Prerequisites
- [Rust & Cargo](https://rustup.rs/) (1.75+ recommended)
- **Windows**: [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (`link.exe`)
- **Linux**: Standard development libraries:
  ```bash
  sudo apt update && sudo apt install -y pkg-config libglib2.0-dev libgtk-3-dev libx11-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev