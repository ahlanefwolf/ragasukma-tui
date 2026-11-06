# Tab 1 — Agent Workspace

Hotkeys for the **Agent** tab (AI chat + tools + workspace panels).
Source of truth: `src/agent_view/controller.rs`, footer: `src/agent_view/ui/mod.rs`.

## Navigation modes (focus areas)

The Agent tab has several focus areas cycled with `Tab` / `Shift+Tab`:

`Folders` → `Tasks` → `Files` → `Chat` → `Input` → `McpTools` → `ModelTelemetry` → (loop)

When the **Input textarea** is focused, most single-letter keys are typed instead
of triggering actions. Use `Esc` (or `Tab` back to panels) to leave the input.

---

## Global (any focus, no modal open)

| Key             | Action                                  |
|-----------------|-----------------------------------------|
| `Tab`           | Focus next panel                        |
| `Shift+Tab`     | Focus previous panel                    |
| `i`             | Jump focus to Input                     |
| `b`             | Toggle sidebar visibility               |
| `m`             | Open Model selector modal               |
| `P` (Shift+p)   | Open Project switcher modal             |
| `F5`            | Refresh models (telemetry)              |
| `x`             | Toggle expand/collapse tool output card |
| `q`             | Quit app                                |
| `Ctrl+C`        | Quit app                                |
| `Alt+s`         | Cancel running turn (if running)        |

> **Note on `q`:** `q` quits the app only when no input/modal is focused.
> While any text input (Input textarea, a modal form, tag query) is active,
> press `Esc` first to leave it — otherwise `q` is typed as a character.

---

## Folders focus

| Key       | Action                          |
|-----------|---------------------------------|
| `j` / `↓` | Next folder                     |
| `k` / `↑` | Previous folder                 |
| `a`       | Open "Add Folder" modal         |
| `d`       | Remove active folder            |
| `o`       | Reveal folder in file manager   |

## Tasks focus

| Key       | Action                            |
|-----------|-----------------------------------|
| `j` / `↓` | Next task                         |
| `k` / `↑` | Previous task                     |
| `Enter`   | Switch to selected task           |
| `n`       | New task                          |
| `d`       | Delete selected task              |
| `s`       | Switch sidebar tab (Tasks/Files)  |

## Files focus (file tree)

| Key       | Action                            |
|-----------|-----------------------------------|
| `j` / `↓` | Next file                         |
| `k` / `↑` | Previous file                     |
| `Enter`   | Toggle folder expand / open file  |
| `a`       | Open "Add Folder" modal           |
| `o`       | Reveal in file manager            |
| `r`       | Rescan workspace files            |
| `s`       | Switch sidebar tab (Tasks/Files)  |

## Chat focus (conversation log)

| Key       | Action                |
|-----------|-----------------------|
| `j` / `↓` | Scroll chat down      |
| `k` / `↑` | Scroll chat up        |

## Input focus (textarea)

| Key            | Action                              |
|----------------|-------------------------------------|
| `Enter`        | Submit prompt                       |
| `Alt+Enter`    | Insert newline                      |
| `Esc`          | Close tag query (if active)         |
| `Ctrl+A`       | Cursor to line start                |
| `Ctrl+E`       | Cursor to line end                  |

### Tag query (`@file` autocomplete popup)
| Key             | Action           |
|-----------------|------------------|
| `↓`             | Next suggestion  |
| `↑`             | Previous suggestion |
| `Tab` / `Enter` | Apply tag        |
| `Esc`           | Close tag query  |

## MCP Tools focus

| Key        | Action                        |
|------------|-------------------------------|
| `j` / `↓`  | Next MCP server / tool        |
| `k` / `↑`  | Previous MCP server / tool    |
| `Space`    | Toggle MCP server enabled     |
| `Enter`    | Toggle MCP server enabled     |
| `t`        | Cycle tool filter             |
| `r`        | Refresh MCP servers           |

## Model Telemetry focus

| Key   | Action           |
|-------|------------------|
| `r`   | Refresh models   |

---

## Modals (each modal traps keys; lists below are the active bindings)

### Add Folder to Workspace (`add_folder_modal.is_open`)
| Key         | Action                    |
|-------------|---------------------------|
| `Enter`     | Submit add-folder         |
| `Tab`       | Open OS folder browser    |
| `Backspace` | Delete char               |
| any char    | Type path                 |
| `Esc`       | Close modal               |

### Project Switcher (`project_modal.is_open`)
| Key             | Action                    |
|-----------------|---------------------------|
| `j` / `↓`       | Next project              |
| `k` / `↑`       | Previous project          |
| `Enter`         | Switch to selected project|
| `n`             | Open "Create Project" modal |
| `d`             | Delete project            |
| `Esc`           | Close modal               |

### Create Project (`create_project_modal.is_open`)
| Key         | Action             |
|-------------|--------------------|
| `Enter`     | Submit create      |
| `Backspace` | Delete char        |
| any char    | Type project name  |
| `Esc`       | Close modal        |

### Select Model (`model_modal.is_open`)
| Key             | Action                     |
|-----------------|----------------------------|
| `j` / `↓`/`Tab` | Next model                 |
| `k` / `↑`/`Shift+Tab` | Previous model       |
| `Enter`         | Apply model                |
| `Backspace`     | Delete filter char         |
| any char        | Filter models              |
| `Esc`           | Close modal                |
