# Tab 4 — Claude (Accounts & Quotas)

Provider account management for Anthropic Claude. Same controller as Tab 2
(`src/provider_view/controller.rs`), so hotkeys mirror Antigravity/Codex.

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

- Adding a Claude account triggers the OAuth flow (`StartClaudeOAuth`).
