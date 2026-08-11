use std::fmt;

use palette::rgb::{Rgb, Rgba};

use crate::settings::NfoRenderSettings;

pub(crate) const BUILT_IN_THEMES: [NfoThemePreset; 2] =
    [NfoThemePreset::NeonPasture, NfoThemePreset::CobaltPaper];

const MIN_ZOOM_PERCENT: u16 = 50;
const MAX_ZOOM_PERCENT: u16 = 300;
const ZOOM_STEP_PERCENT: u16 = 10;
const BASE_BLOCK_WIDTH: u16 = 7;
const BASE_BLOCK_HEIGHT: u16 = 12;
const BASE_FONT_SIZE: f32 = 14.0;

const NEON_PASTURE: ThemeValues = ThemeValues {
    background_color: Rgb::new(0.004, 0.008, 0.008),
    text_color: Rgba::new(0.96, 0.98, 0.98, 1.0),
    art_color: Rgba::new(0.0, 0.89, 0.92, 1.0),
    glow_enabled: true,
    glow_color: Rgba::new(0.0, 0.08, 0.09, 1.0),
    glow_radius: 24,
    hyperlink_color: Rgba::new(0.04, 0.38, 0.96, 1.0),
    hyperlink_underline: true,
};

const COBALT_PAPER: ThemeValues = ThemeValues {
    background_color: Rgb::new(0.975, 0.98, 0.985),
    text_color: Rgba::new(0.08, 0.085, 0.08, 1.0),
    art_color: Rgba::new(0.10, 0.26, 0.66, 1.0),
    glow_enabled: true,
    glow_color: Rgba::new(0.64, 0.72, 0.96, 1.0),
    glow_radius: 8,
    hyperlink_color: Rgba::new(0.05, 0.20, 0.72, 1.0),
    hyperlink_underline: true,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NfoThemePreset {
    NeonPasture,
    CobaltPaper,
    Custom,
}

impl NfoThemePreset {
    pub(crate) const fn values(self) -> Option<&'static ThemeValues> {
        match self {
            Self::NeonPasture => Some(&NEON_PASTURE),
            Self::CobaltPaper => Some(&COBALT_PAPER),
            Self::Custom => None,
        }
    }
}

impl fmt::Display for NfoThemePreset {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NeonPasture => "Neon Pasture",
            Self::CobaltPaper => "Cobalt Paper",
            Self::Custom => "Custom",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ThemeValues {
    pub(crate) background_color: Rgb,
    pub(crate) text_color: Rgba,
    pub(crate) art_color: Rgba,
    pub(crate) glow_enabled: bool,
    pub(crate) glow_color: Rgba,
    pub(crate) glow_radius: u16,
    pub(crate) hyperlink_color: Rgba,
    pub(crate) hyperlink_underline: bool,
}

impl ThemeValues {
    pub(crate) fn from_render_settings(settings: &NfoRenderSettings) -> Self {
        Self {
            background_color: settings.background_color,
            text_color: settings.text_color,
            art_color: settings.art_color,
            glow_enabled: settings.blur_enabled,
            glow_color: settings.blur_color,
            glow_radius: settings.blur_radius,
            hyperlink_color: settings.hyperlink_color,
            hyperlink_underline: settings.hyperlink_underline,
        }
    }

    pub(crate) fn apply_to_render_settings(self, settings: &mut NfoRenderSettings) {
        settings.background_color = self.background_color;
        settings.text_color = self.text_color;
        settings.art_color = self.art_color;
        settings.blur_enabled = self.glow_enabled;
        settings.blur_color = self.glow_color;
        settings.blur_radius = self.glow_radius;
        settings.hyperlink_color = self.hyperlink_color;
        settings.hyperlink_underline = self.hyperlink_underline;
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PresentationState {
    pub(crate) selected_theme: NfoThemePreset,
    pub(crate) theme_values: ThemeValues,
    pub(crate) inspector_open: bool,
    pub(crate) overflow_open: bool,
    pub(crate) about_open: bool,
    pub(crate) zoom_percent: u16,
    pub(crate) use_ansi_colors: bool,
    pub(crate) line_wrapping: bool,
    pub(crate) antialiasing: bool,
    pub(crate) character_ratio: f32,
}

impl PresentationState {
    pub(crate) const fn new() -> Self {
        Self {
            selected_theme: NfoThemePreset::NeonPasture,
            theme_values: NEON_PASTURE,
            inspector_open: true,
            overflow_open: false,
            about_open: false,
            zoom_percent: 100,
            use_ansi_colors: true,
            line_wrapping: false,
            antialiasing: true,
            character_ratio: BASE_BLOCK_WIDTH as f32 / BASE_BLOCK_HEIGHT as f32,
        }
    }

    pub(crate) fn select_theme(
        &mut self,
        preset: NfoThemePreset,
        settings: &mut NfoRenderSettings,
    ) {
        self.selected_theme = preset;

        if let Some(values) = preset.values() {
            self.theme_values = *values;
            self.theme_values.apply_to_render_settings(settings);
        } else {
            self.theme_values = ThemeValues::from_render_settings(settings);
        }
    }

    pub(crate) fn mark_custom(&mut self) {
        self.selected_theme = NfoThemePreset::Custom;
    }

    pub(crate) fn zoom_in(&mut self) {
        self.zoom_percent = self
            .zoom_percent
            .saturating_add(ZOOM_STEP_PERCENT)
            .min(MAX_ZOOM_PERCENT);
    }

    pub(crate) fn zoom_out(&mut self) {
        self.zoom_percent = self
            .zoom_percent
            .saturating_sub(ZOOM_STEP_PERCENT)
            .max(MIN_ZOOM_PERCENT);
    }

    pub(crate) fn apply_zoom(&self, settings: &mut NfoRenderSettings) {
        settings.enhanced_view_block_height =
            scaled_dimension(BASE_BLOCK_HEIGHT, self.zoom_percent);
        settings.enhanced_view_block_width = (f32::from(settings.enhanced_view_block_height)
            * self.character_ratio)
            .round()
            .max(1.0) as u16;
        settings.classic_font_size = BASE_FONT_SIZE * f32::from(self.zoom_percent) / 100.0;
    }
}

impl Default for PresentationState {
    fn default() -> Self {
        Self::new()
    }
}

fn scaled_dimension(base: u16, percent: u16) -> u16 {
    (base * percent + 50) / 100
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_values_map_to_and_from_render_settings() {
        let mut settings = NfoRenderSettings::default();
        let mut presentation = PresentationState::new();

        presentation.select_theme(NfoThemePreset::CobaltPaper, &mut settings);

        let expected = *NfoThemePreset::CobaltPaper.values().unwrap();
        assert_eq!(presentation.theme_values, expected);
        assert_eq!(ThemeValues::from_render_settings(&settings), expected);
        assert_eq!(settings.blur_enabled, expected.glow_enabled);
        assert_eq!(settings.blur_color, expected.glow_color);
        assert_eq!(settings.blur_radius, expected.glow_radius);
    }

    #[test]
    fn edited_theme_transitions_to_custom() {
        let mut presentation = PresentationState::new();
        assert_eq!(presentation.selected_theme, NfoThemePreset::NeonPasture);

        presentation.mark_custom();

        assert_eq!(presentation.selected_theme, NfoThemePreset::Custom);
        assert!(presentation.selected_theme.values().is_none());
    }

    #[test]
    fn zoom_is_clamped_and_applies_from_base_dimensions() {
        let mut presentation = PresentationState::new();
        let mut settings = NfoRenderSettings::default();

        for _ in 0..100 {
            presentation.zoom_in();
        }
        assert_eq!(presentation.zoom_percent, MAX_ZOOM_PERCENT);
        presentation.apply_zoom(&mut settings);
        assert_eq!(settings.enhanced_view_block_width, 21);
        assert_eq!(settings.enhanced_view_block_height, 36);
        assert_eq!(settings.classic_font_size, 42.0);

        for _ in 0..100 {
            presentation.zoom_out();
        }
        assert_eq!(presentation.zoom_percent, MIN_ZOOM_PERCENT);
        presentation.apply_zoom(&mut settings);
        assert_eq!(settings.enhanced_view_block_width, 4);
        assert_eq!(settings.enhanced_view_block_height, 6);
        assert_eq!(settings.classic_font_size, 7.0);
    }

    #[test]
    fn zoom_uses_ten_percent_intermediate_steps() {
        let mut presentation = PresentationState::new();
        let mut settings = NfoRenderSettings::default();

        presentation.zoom_in();
        assert_eq!(presentation.zoom_percent, 110);
        presentation.apply_zoom(&mut settings);
        assert_eq!(settings.enhanced_view_block_width, 8);
        assert_eq!(settings.enhanced_view_block_height, 13);
        assert!((settings.classic_font_size - 15.4).abs() < 0.001);

        presentation.zoom_in();
        assert_eq!(presentation.zoom_percent, 120);
        presentation.zoom_out();
        assert_eq!(presentation.zoom_percent, 110);
    }
}
