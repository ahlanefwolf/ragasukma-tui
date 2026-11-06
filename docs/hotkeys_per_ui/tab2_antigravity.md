# Tab 2 — Antigravity (Accounts & Quotas)

Provider account management for Google Antigravity. Source: `src/provider_view/controller.rs`, footer: `src/provider_view/ui.rs`.

## Focus areas

Toggle with `Tab` / `Shift+Tab`: `Accounts` ⇄ `Quotas`.

## Accounts focus

| Key       | Action                         |
|-----------|--------------------------------|
| `j` / `k` | Select account (prev/next)     |
| `Space`   | Set as active account          |
| `a`       | Add account (OAuth)            |
| `d`       | Delete account                 |
| `r`       | Refresh accounts               |
| `Tab`     | Switch to Quotas               |

## Quotas focus

| Key       | Action                         |
|-----------|--------------------------------|
| `j` / `k` | Scroll quotas (prev/next)      |
| `r`       | Refresh quota info             |
| `Tab`     | Switch back to Accounts        |

## Notes

- Adding an Antigravity account triggers the OAuth flow (`StartAntigravityOAuth`).
- Modal/dialog keystrokes (if an OAuth/browser prompt appears) follow the global
  form bindings: `Enter` confirm, `Esc` cancel.
