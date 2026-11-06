# Tab 5 — Custom LLM

Manage custom LLM providers. Source: `src/custom_llm_view/controller.rs`,
footer: `src/custom_llm_view/ui.rs`.

## List mode (no modal open)

| Key             | Action                       |
|-----------------|------------------------------|
| `j` / `k`       | Navigate (prev/next)         |
| `Space`/`Enter` | Toggle enabled               |
| `a` / `n`       | Add new provider             |
| `e`             | Edit selected provider       |
| `d`             | Delete selected provider     |
| `r`             | Refresh list                 |
| `h` / `l`       | Horizontal scroll (columns)  |

## Add / Edit form modal

| Key     | Action         |
|---------|----------------|
| `Tab`   | Next field     |
| `Enter` | Submit         |
| `Esc`   | Cancel         |

## Delete confirm modal

| Key       | Action   |
|-----------|----------|
| `y`/`Enter` | Confirm delete |
| `n` / `Esc` | Cancel         |

## Notes

- Footer hints adapt to state: delete-confirm > form-open > list.
