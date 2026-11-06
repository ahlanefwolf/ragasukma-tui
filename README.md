<div align="center">

  <img src="icons/128x128@2x.png" alt="Ragasukma Logo" width="120" height="120" />

  # Ragasukma TUI
  **Your AI Coding Assistant, Right in the Terminal**

  [![Platform](https://img.shields.io/badge/Platform-Linux_%7C_Windows_%7C_macOS-2F3B46?style=flat-square)](https://github.com/ahlanefwolf/ragasukma-tui/releases)
  [![License: MIT](https://img.shields.io/badge/License-MIT-F2B5D4?style=flat-square)](LICENSE)
  [![Donate](https://img.shields.io/badge/Donate-SociaBuzz-FF7043?style=flat-square&logo=coffeescript&logoColor=white)](https://sociabuzz.com/ahlanefwolf)

  <p align="center">
    Chat with AI to get your code done, keep several AI accounts side by side,
    and watch your remaining quota — all from the terminal. No browser, no mouse.
  </p>

</div>

---

## 💡 What is Ragasukma TUI?

Imagine having a programming assistant sitting right next to you. You just say
_"please fix the bug on the login page"_, and it reads the code, changes it, and makes
sure everything still works.

**Ragasukma TUI is the desk where that assistant works.** Everything happens in your
terminal — light, fast, no heavy app to open.

Think of it like a car:
- **Ragasukma Core** = the engine 🚗
- **Ragasukma TUI** = the dashboard 🕹️

You just steer. The engine is already working behind the scenes.

---

## ✨ What It Can Do

### 🤖 Chat and Get Code Done
- Ask questions about your code straight from the terminal.
- Let the AI fix bugs, add features, or clean up your code.
- Point it at a specific file with `@` so it knows exactly what you mean.

### 👀 See It First, Approve It After
- Before the AI changes a file, you **see the changes first**.
- Press `y` to approve, `n` to reject. You stay in full control.
- No surprises — your code never changes without your permission.

### 📁 Work on Several Projects at Once
- Add as many folders as you like to one workspace.
- Switch between projects without closing and reopening the app.

### 🔑 One-Click Account Login
- Press `a` on any AI account tab and your browser opens for you.
- Log in, and the account is connected. Done.
- Works with **Claude**, **ChatGPT / Codex**, and **Google Antigravity**.

### 📊 Track Quota and Switch Accounts
- See your remaining quota, daily limits, and account status on one screen.
- Have several accounts? Switch between them with `Enter` or `Space`.

### 🔌 Bring Your Own AI
- Use your own models — local ones like **Ollama**, or hosted ones like
  **OpenRouter**, **DeepSeek**, **vLLM**, and more.
- Just type in the address and key on the Custom LLM tab. No code changes needed.

### 🧩 Add New Abilities
- Plug in extra tools: web browsing, database access, and plenty more.
- Set everything up from the MCP Servers tab — try a tool, test it, and connect it
  without ever opening a config file.

### 🧠 Control How Much the AI Thinks
- Show or hide the AI's thinking process (`r`).
- Show or hide the details of the tools it's using (`x`).
- Want everything on screen, or a clean view? Your call.

### ⚡ Light and Fast
- Starts instantly and sips almost no memory.
- Everything works from the keyboard. No mouse required.

---

## 🚀 Getting Started

### What you'll need
- **Rust** version 1.85 or newer — [how to install](https://rustup.rs)
- **Git** (recommended)
- Linux, macOS, or Windows

No Rust yet? Paste this into your terminal:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Option 1 — Try It Right Away

```bash
git clone https://github.com/ahlanefwolf/ragasukma-tui.git
cd ragasukma-tui
cargo run --bin ragasukma-tui
```

The first run takes a few minutes while everything is prepared.
After that, opening it again is a matter of seconds.

### Option 2 — Build the Everyday Version (Recommended)

Faster and lighter — the one to use day to day:

```bash
cargo build --release --bin ragasukma-tui
./target/release/ragasukma-tui
```

You'll find a single runnable file at `target/release/ragasukma-tui`.
Move it anywhere you like — it runs on its own.

### First Steps Once It Opens

1. Press `1` to open the **Agent** tab.
2. Press `a` to add your project folder.
3. Press `i` to start typing a question or command.
4. Press `Enter` to send it.

That's it. The AI is ready to work.

---

## 🕹️ Most-Used Keys

| Key | What It Does |
| :--- | :--- |
| `1` – `6` | Jump to a tab: Agent, Antigravity, Codex, Claude, Custom LLM, MCP Servers |
| `[` / `]` | Move to the previous / next tab |
| `i` | Start typing in the message box |
| `Enter` | Send a message / select an item |
| `Esc` | Cancel typing or close an open dialog |
| `Tab` | Move between panels |
| `m` | Choose an AI model |
| `a` | Log in to a new account via browser / add a project folder |
| `r` | Show or hide the AI's thinking / refresh quota |
| `x` | Show or hide tool usage details |
| `y` / `n` | Approve / reject file changes |
| `q` | Quit the application |
| `Ctrl+C` | Stop the AI while it's working, or quit |

> **Tip:** all your data (chat history, accounts, and progress) is saved automatically.
> Close the app whenever you like — open it again tomorrow and everything is still there.

---

## 🤝 Need Help?

- 🖥️ **Want the engine inside your own project?** See [Ragasukma Core](../ragasukma-core).
- 🐛 **Found a problem?** Report it in [Issues](https://github.com/ahlanefwolf/ragasukma-tui/issues).
- ❤️ **Found it useful?** You can [buy me a coffee](https://sociabuzz.com/ahlanefwolf).

---

## 📄 License

Released under the [MIT License](LICENSE) — free to use, modify, and share.

Made by **Ahlan Saputra** ([@ahlanefwolf](https://github.com/ahlanefwolf)).
Copyright © 2026 Ahlan Saputra.
