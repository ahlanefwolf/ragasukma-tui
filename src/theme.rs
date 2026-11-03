use ratatui::style::{Color, Modifier, Style};

/// Central Theme definition for Ragasukma TUI
/// Mapped accurately from `@ai-voice/web` (Genshin Impact / Dark Slate & Tech Gold design system)
pub struct Theme;

impl Theme {
    // -------------------------------------------------------------
    // Surfaces & Backgrounds (Hex matching @ai-voice/web CSS variables)
    // -------------------------------------------------------------
    pub const BG_CANVAS: Color = Color::Rgb(15, 18, 24); // #0f1218 --gi-bg-canvas
    pub const BG_SURFACE: Color = Color::Rgb(24, 29, 38); // #181d26 --gi-bg-surface
    pub const BG_SURFACE_ELEVATED: Color = Color::Rgb(34, 41, 54); // #222936 --gi-bg-surface-elevated
    pub const BORDER_SUBTLE: Color = Color::Rgb(51, 60, 77); // #333c4d --gi-border-subtle
    pub const BORDER_GOLD: Color = Color::Rgb(158, 128, 70); // #9e8046 --gi-border-gold

    // -------------------------------------------------------------
    // Typography Colors
    // -------------------------------------------------------------
    pub const TEXT_PRIMARY: Color = Color::Rgb(237, 234, 225); // #edeae1 --gi-text-primary
    pub const TEXT_SECONDARY: Color = Color::Rgb(159, 167, 184); // #9fa7b8 --gi-text-secondary
    pub const TEXT_MUTED: Color = Color::Rgb(95, 102, 117); // #5f6675 --gi-text-muted

    // -------------------------------------------------------------
    // Brand Colors
    // -------------------------------------------------------------
    pub const GOLD_PRIMARY: Color = Color::Rgb(229, 184, 92); // #e5b85c --gi-primary
    pub const GOLD_HOVER: Color = Color::Rgb(243, 206, 122); // #f3ce7a --gi-primary-hover
    pub const GOLD_ACTIVE: Color = Color::Rgb(199, 154, 62); // #c79a3e --gi-primary-active
    pub const GOLD_FOREGROUND: Color = Color::Rgb(17, 19, 24); // #111318 text on primary

    pub const CYAN_ACCENT: Color = Color::Rgb(79, 195, 247); // #4fc3f7 --gi-accent
    pub const CYAN_HOVER: Color = Color::Rgb(124, 212, 253); // #7cd4fd --gi-accent-hover
    pub const CYAN_ACTIVE: Color = Color::Rgb(43, 166, 222); // #2ba6de --gi-accent-active

    // -------------------------------------------------------------
    // Status Indicators
    // -------------------------------------------------------------
    pub const STATUS_SUCCESS: Color = Color::Rgb(69, 184, 118); // #45b876 --gi-status-success
    pub const STATUS_WARNING: Color = Color::Rgb(224, 159, 62); // #e09f3e --gi-status-warning
    pub const STATUS_DANGER: Color = Color::Rgb(224, 83, 69); // #e05345 --gi-status-danger
    pub const STATUS_ERROR: Color = Self::STATUS_DANGER;
    pub const STATUS_INFO: Color = Color::Rgb(62, 153, 224); // #3e99e0 --gi-status-info

    // -------------------------------------------------------------
    // ratatui Style Builders
    // -------------------------------------------------------------
    #[must_use]
    pub fn block_base() -> Style {
        Style::default().bg(Self::BG_SURFACE).fg(Self::TEXT_PRIMARY)
    }

    #[must_use]
    pub fn card_base() -> Style {
        Style::default()
            .bg(Self::BG_SURFACE_ELEVATED)
            .fg(Self::TEXT_PRIMARY)
    }

    #[must_use]
    pub fn border_subtle() -> Style {
        Style::default().fg(Self::BORDER_SUBTLE)
    }

    #[must_use]
    pub fn border_gold() -> Style {
        Style::default().fg(Self::BORDER_GOLD)
    }

    #[must_use]
    pub fn border_focused() -> Style {
        Style::default().fg(Self::BORDER_GOLD)
    }

    #[must_use]
    pub fn header_title() -> Style {
        Style::default()
            .fg(Self::GOLD_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn tab_active() -> Style {
        Style::default()
            .bg(Self::GOLD_PRIMARY)
            .fg(Self::GOLD_FOREGROUND)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn tab_inactive() -> Style {
        Style::default()
            .bg(Self::BG_SURFACE)
            .fg(Self::TEXT_SECONDARY)
    }

    #[must_use]
    pub fn badge_gold() -> Style {
        Style::default()
            .bg(Self::GOLD_PRIMARY)
            .fg(Self::GOLD_FOREGROUND)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn badge_cyan() -> Style {
        Style::default()
            .bg(Self::CYAN_ACCENT)
            .fg(Self::GOLD_FOREGROUND)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn badge_info() -> Style {
        Self::badge_cyan()
    }

    #[must_use]
    pub fn badge_gray() -> Style {
        Style::default()
            .bg(Self::BG_SURFACE_ELEVATED)
            .fg(Self::TEXT_SECONDARY)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn block_surface() -> Style {
        Style::default().bg(Self::BG_SURFACE_ELEVATED).fg(Self::TEXT_PRIMARY)
    }

    #[must_use]
    pub fn key_badge() -> Style {
        Style::default()
            .bg(Self::BG_SURFACE_ELEVATED)
            .fg(Self::CYAN_ACCENT)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn status_success() -> Style {
        Style::default()
            .fg(Self::STATUS_SUCCESS)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn status_warning() -> Style {
        Style::default()
            .fg(Self::STATUS_WARNING)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn status_danger() -> Style {
        Style::default()
            .fg(Self::STATUS_DANGER)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn status_error() -> Style {
        Self::status_danger()
    }

    #[must_use]
    pub fn status_info() -> Style {
        Style::default()
            .fg(Self::STATUS_INFO)
            .add_modifier(Modifier::BOLD)
    }

    #[must_use]
    pub fn status_inactive() -> Style {
        Style::default().fg(Self::TEXT_MUTED)
    }

    #[must_use]
    pub fn text_primary() -> Style {
        Style::default().fg(Self::TEXT_PRIMARY)
    }

    #[must_use]
    pub fn text_secondary() -> Style {
        Style::default().fg(Self::TEXT_SECONDARY)
    }

    #[must_use]
    pub fn text_muted() -> Style {
        Style::default().fg(Self::TEXT_MUTED)
    }

    #[must_use]
    pub fn text_gold() -> Style {
        Style::default().fg(Self::GOLD_PRIMARY)
    }

    #[must_use]
    pub fn text_cyan() -> Style {
        Style::default().fg(Self::CYAN_ACCENT)
    }

    #[must_use]
    pub fn list_item_selected() -> Style {
        Style::default()
            .bg(Self::BG_SURFACE_ELEVATED)
            .fg(Self::GOLD_PRIMARY)
            .add_modifier(Modifier::BOLD)
    }
}
