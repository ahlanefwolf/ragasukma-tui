# Ragasukma TUI 🖥️⚡

[![License: MIT](https://img.shields.io/badge/License-MIT-F2B5D4?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Linux_%7C_Windows_%7C_macOS-2F3B46?style=flat-square)](https://github.com/ahlanefwolf/ragasukma-tui/releases)
[![Donate](https://img.shields.io/badge/Donate-SociaBuzz-FF7043?style=flat-square&logo=coffeescript&logoColor=white)](https://sociabuzz.com/ahlanefwolf)

> A smart Terminal User Interface (TUI) for coding alongside AI agents, managing multi-account LLMs (Claude, ChatGPT/Codex, Google Antigravity), and monitoring quota usage directly from your terminal.

---

## ✨ Key Features

- 🤖 **AI Coding Workspace**:
  - Chat directly with AI agents inside your terminal.
  - Review file changes (diff preview) before the AI applies edits (`y` to approve, `n` to reject).
  - Manage multiple projects/folders simultaneously in a single workspace.
  - Toggle AI thinking process (*reasoning*) and tool execution details on or off.
  - Easily tag files and context using the `@` symbol in the prompt input.
- 🔑 **Effortless Account Login (1-Click OAuth)**:
  - Simply press `a` in any provider tab to sign in via your browser; accounts link automatically (supports **Claude**, **Codex / ChatGPT**, and **Antigravity**).
- 📊 **Quota Monitoring & Multi-Account Switching**:
  - Visually inspect remaining quotas, daily/minute limits, and account status.
  - Switch active accounts instantly with `Enter` or `Space`.
- 🔌 **Custom AI Models (Custom LLMs)**:
  - Bring your own models (local Ollama, OpenRouter, DeepSeek, vLLM, etc.) by configuring endpoints and API keys in Tab 5.
- ⚡ **Lightweight & Blazing Fast**: Built with Rust, ultra-low memory footprint, and full keyboard-driven navigation—no mouse needed.

---

## 🕹️ Essential Hotkeys & Navigation

| Key | Description |
| :--- | :--- |
| `1` - `5` | Switch tabs (`1`: Agent Chat, `2`: Antigravity, `3`: Codex, `4`: Claude, `5`: Custom LLM) |
| `i` | Focus prompt input to start typing |
| `Enter` | Send prompt / select item |
| `Esc` | Unfocus input field or close open popups / modals |
| `Tab` | Cycle focus between panels |
| `m` | Open model selector modal (Claude, GPT, etc.) |
| `a` | **Login new account via browser** (in provider tabs) or add folder (in agent tab) |
| `r` | Toggle AI reasoning visibility or refresh quota data |
| `x` | Toggle tool execution details |
| `?` or `F1` | Open in-app help cheatsheet |
| `Ctrl+C` | Cancel ongoing AI turn or exit application |

---

## 🚀 Getting Started

### 1. Run Directly
```bash
cargo run --bin ragasukma-tui
```

### 2. Build Release Binary
```bash
cargo build --release --bin ragasukma-tui
```
The compiled binary will be available at `target/release/ragasukma-tui`.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE) — see the [LICENSE](LICENSE) file for details.

Created by **Ahlan Saputra** ([@ahlanefwolf](https://github.com/ahlanefwolf)).  
Copyright © 2026 Ahlan Saputra.
