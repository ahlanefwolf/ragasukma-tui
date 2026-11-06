# Tab 6 — MCP Servers

Manage registered MCP servers. Source: `src/mcp_server_view/controller.rs`,
footer: `src/mcp_server_view/ui.rs`.

## List mode (no modal open)

| Key             | Action                       |
|-----------------|------------------------------|
| `j` / `k`       | Navigate (prev/next)         |
| `Space`/`Enter` | Toggle enabled               |
| `a`             | Add new server               |
| `e`             | Edit selected server         |
| `d`             | Delete selected server       |
| `r`             | Refresh list                 |

## Add / Edit form modal

| Key     | Action         |
|---------|----------------|
| `Tab`   | Next field     |
| `Enter` | Submit         |
| `Esc`   | Cancel         |

## Delete confirm modal

| Key         | Action         |
|-------------|----------------|
| `y`/`Enter` | Confirm delete |
| `n` / `Esc` | Cancel         |

## Notes

- Empty state shows: "No MCP servers registered. Press [a] to add a new server."
- Footer hints adapt: delete-confirm > form-open > list.
