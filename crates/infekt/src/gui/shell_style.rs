#![allow(
    dead_code,
    reason = "the shared style catalog is intentionally consumed incrementally by shell widgets"
)]

//! Styling primitives for the modern application shell.
//!
//! The colors in this module intentionally remain translucent. They are meant
//! to be layered over the active NFO theme instead of behaving like a separate
//! application theme.

use std::sync::Arc;

use iced::gradient;
use iced::overlay::menu;
use iced::widget::{button, container, pick_list, slider, toggler};
use iced::{Background, Border, Color, Shadow, Theme, Vector};
use palette::rgb::{Rgb, Rgba};
use palette::{FromColor, Hsl};

use crate::settings::NfoRenderSettings;

/// The small, shared palette used by every surface in the application shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShellTokens {
    pub(crate) is_dark: bool,
    pub(crate) root: Color,
    pub(crate) canvas: Color,
    pub(crate) toolbar: Color,
    pub(crate) glass: Color,
    pub(crate) raised: Color,
    pub(crate) input: Color,
    pub(crate) border: Color,
    pub(crate) text: Color,
    pub(crate) text_muted: Color,
    pub(crate) accent: Color,
    pub(crate) secondary_accent: Color,
    pub(crate) disabled: Color,
    pub(crate) control_radius: f32,
    pub(crate) panel_radius: f32,
    pub(crate) shadow: Shadow,
}

impl ShellTokens {
    pub(crate) fn from_settings(settings: &NfoRenderSettings) -> Self {
        let background = from_rgb(settings.background_color);
        let text = from_rgba(settings.text_color);
        let accent = from_rgba(settings.art_color);
        let secondary_accent = from_rgba(settings.hyperlink_color);
        let is_dark = is_dark(settings.background_color);

        if is_dark {
            Self {
                is_dark,
                root: mix(background, Color::BLACK, 0.12),
                canvas: mix(background, accent, 0.025),
                toolbar: layer(Color::WHITE, 0.055),
                glass: layer(Color::WHITE, 0.075),
                raised: layer(Color::WHITE, 0.12),
                input: layer(Color::BLACK, 0.20),
                border: layer(Color::WHITE, 0.15),
                text,
                text_muted: fade(text, 0.62),
                accent,
                secondary_accent,
                disabled: fade(text, 0.34),
                control_radius: 8.0,
                panel_radius: 14.0,
                shadow: Shadow {
                    color: layer(Color::BLACK, 0.42),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 24.0,
                },
            }
        } else {
            Self {
                is_dark,
                root: mix(background, Color::BLACK, 0.015),
                canvas: mix(background, accent, 0.025),
                toolbar: layer(Color::WHITE, 0.62),
                glass: layer(Color::WHITE, 0.72),
                raised: layer(Color::WHITE, 0.90),
                input: Color::from_rgba(0.12, 0.16, 0.20, 0.045),
                border: layer(Color::BLACK, 0.10),
                text,
                text_muted: fade(text, 0.60),
                accent,
                secondary_accent,
                disabled: fade(text, 0.32),
                control_radius: 8.0,
                panel_radius: 14.0,
                shadow: Shadow {
                    color: layer(Color::BLACK, 0.14),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 28.0,
                },
            }
        }
    }

    fn control_border(self, color: Color) -> Border {
        Border {
            color,
            width: 0.75,
            radius: self.control_radius.into(),
        }
    }

    fn panel_border(self, color: Color) -> Border {
        Border {
            color,
            width: 0.75,
            radius: self.panel_radius.into(),
        }
    }
}

impl From<&NfoRenderSettings> for ShellTokens {
    fn from(settings: &NfoRenderSettings) -> Self {
        Self::from_settings(settings)
    }
}

impl From<&Arc<NfoRenderSettings>> for ShellTokens {
    fn from(settings: &Arc<NfoRenderSettings>) -> Self {
        Self::from_settings(settings.as_ref())
    }
}

impl From<Arc<NfoRenderSettings>> for ShellTokens {
    fn from(settings: Arc<NfoRenderSettings>) -> Self {
        Self::from_settings(settings.as_ref())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SurfaceRole {
    Root,
    Canvas,
    BackdropCanvas,
    Toolbar,
    BackdropToolbar,
    Inspector,
    BackdropInspector,
    Glass,
    NavigatorGlass,
    ModalGlass,
    Raised,
    Input,
}

/// Produces a closure suitable for [`iced::widget::Container::style`].
pub(crate) fn surface(
    tokens: ShellTokens,
    role: SurfaceRole,
) -> impl Fn(&Theme) -> container::Style + Clone {
    move |_theme| surface_appearance(tokens, role)
}

fn surface_appearance(tokens: ShellTokens, role: SurfaceRole) -> container::Style {
    let (background, border, shadow): (Background, Border, Shadow) = match role {
        SurfaceRole::Root => (tokens.root.into(), Border::default(), Shadow::default()),
        SurfaceRole::Canvas => (tokens.canvas.into(), Border::default(), Shadow::default()),
        SurfaceRole::BackdropCanvas => (
            layer(tokens.canvas, if tokens.is_dark { 0.78 } else { 0.80 }).into(),
            Border::default(),
            Shadow::default(),
        ),
        SurfaceRole::Toolbar => (
            tokens.toolbar.into(),
            Border {
                color: tokens.border,
                width: 0.75,
                radius: 0.0.into(),
            },
            Shadow::default(),
        ),
        SurfaceRole::BackdropToolbar => (
            layer(
                composite(tokens.toolbar, tokens.root),
                if tokens.is_dark { 0.86 } else { 0.92 },
            )
            .into(),
            Border {
                color: tokens.border,
                width: 0.75,
                radius: 0.0.into(),
            },
            Shadow::default(),
        ),
        SurfaceRole::Inspector => (
            gradient::Linear::new(2.15)
                .add_stop(
                    0.0,
                    mix(
                        tokens.glass,
                        tokens.secondary_accent,
                        if tokens.is_dark { 0.16 } else { 0.07 },
                    ),
                )
                .add_stop(0.52, tokens.glass)
                .add_stop(
                    1.0,
                    mix(
                        tokens.glass,
                        tokens.accent,
                        if tokens.is_dark { 0.10 } else { 0.04 },
                    ),
                )
                .into(),
            Border {
                color: tokens.border,
                width: 0.75,
                radius: 0.0.into(),
            },
            Shadow {
                offset: Vector::new(-3.0, 0.0),
                blur_radius: 18.0,
                ..tokens.shadow
            },
        ),
        SurfaceRole::BackdropInspector => {
            let opacity = if tokens.is_dark { 0.64 } else { 0.82 };
            let glass = composite(tokens.glass, tokens.root);

            (
                gradient::Linear::new(2.15)
                    .add_stop(0.0, layer(glass, opacity))
                    .add_stop(0.52, layer(glass, opacity))
                    .add_stop(
                        1.0,
                        layer(
                            mix(
                                glass,
                                tokens.accent,
                                if tokens.is_dark { 0.035 } else { 0.015 },
                            ),
                            opacity,
                        ),
                    )
                    .into(),
                Border {
                    color: tokens.border,
                    width: 0.75,
                    radius: 0.0.into(),
                },
                Shadow {
                    offset: Vector::new(-3.0, 0.0),
                    blur_radius: 18.0,
                    ..tokens.shadow
                },
            )
        }
        SurfaceRole::Glass => (
            overlay_glass(tokens).into(),
            tokens.panel_border(tokens.border),
            tokens.shadow,
        ),
        SurfaceRole::NavigatorGlass => (
            overlay_glass(tokens).into(),
            tokens.panel_border(tokens.border),
            Shadow {
                offset: Vector::new(0.0, 10.0),
                blur_radius: 30.0,
                ..tokens.shadow
            },
        ),
        SurfaceRole::ModalGlass => {
            let base = composite(tokens.raised, tokens.root);
            let highlight = mix(
                base,
                tokens.secondary_accent,
                if tokens.is_dark { 0.10 } else { 0.035 },
            );
            let tint = mix(
                base,
                tokens.accent,
                if tokens.is_dark { 0.07 } else { 0.025 },
            );

            (
                gradient::Linear::new(2.15)
                    .add_stop(0.0, highlight)
                    .add_stop(0.46, base)
                    .add_stop(1.0, tint)
                    .into(),
                Border {
                    color: mix(
                        tokens.border,
                        tokens.accent,
                        if tokens.is_dark { 0.20 } else { 0.08 },
                    ),
                    width: 1.0,
                    radius: tokens.panel_radius.into(),
                },
                Shadow {
                    offset: Vector::new(0.0, 14.0),
                    blur_radius: 38.0,
                    ..tokens.shadow
                },
            )
        }
        SurfaceRole::Raised => (
            tokens.raised.into(),
            tokens.control_border(tokens.border),
            Shadow {
                offset: Vector::new(0.0, 2.0),
                blur_radius: 8.0,
                ..tokens.shadow
            },
        ),
        SurfaceRole::Input => (
            tokens.input.into(),
            tokens.control_border(tokens.border),
            Shadow::default(),
        ),
    };

    container::Style::default()
        .color(tokens.text)
        .background(background)
        .border(border)
        .shadow(shadow)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonRole {
    Toolbar,
    Glass,
    Accent,
    Segmented { selected: bool },
}

/// Produces a closure suitable for [`iced::widget::Button::style`].
pub(crate) fn button_style(
    tokens: ShellTokens,
    role: ButtonRole,
) -> impl Fn(&Theme, button::Status) -> button::Style + Clone {
    move |_theme, status| button_appearance(tokens, role, status)
}

fn button_appearance(
    tokens: ShellTokens,
    role: ButtonRole,
    status: button::Status,
) -> button::Style {
    let transparent = Color::TRANSPARENT;
    let (background, text_color, border_color, border_width, shadow) = match role {
        ButtonRole::Toolbar => match status {
            button::Status::Active => (None, tokens.text, transparent, 0.0, Shadow::default()),
            button::Status::Hovered => (
                Some(tokens.raised),
                tokens.text,
                tokens.border,
                0.75,
                Shadow::default(),
            ),
            button::Status::Pressed => (
                Some(tokens.input),
                tokens.text,
                tokens.border,
                0.75,
                Shadow::default(),
            ),
            button::Status::Disabled => {
                (None, tokens.disabled, transparent, 0.0, Shadow::default())
            }
        },
        ButtonRole::Glass => match status {
            button::Status::Active => (
                Some(tokens.glass),
                tokens.text,
                tokens.border,
                0.75,
                Shadow::default(),
            ),
            button::Status::Hovered => (
                Some(tokens.raised),
                tokens.text,
                tokens.border,
                0.75,
                Shadow::default(),
            ),
            button::Status::Pressed => (
                Some(tokens.input),
                tokens.text,
                tokens.border,
                0.75,
                Shadow::default(),
            ),
            button::Status::Disabled => (
                Some(fade(tokens.glass, 0.55)),
                tokens.disabled,
                fade(tokens.border, 0.55),
                0.75,
                Shadow::default(),
            ),
        },
        ButtonRole::Accent => {
            let accent_text = contrast_color(tokens.accent);
            match status {
                button::Status::Active => (
                    Some(tokens.accent),
                    accent_text,
                    fade(tokens.accent, 0.85),
                    0.75,
                    Shadow {
                        offset: Vector::new(0.0, 2.0),
                        blur_radius: 8.0,
                        ..tokens.shadow
                    },
                ),
                button::Status::Hovered => (
                    Some(tone(tokens.accent, tokens.is_dark, 0.10)),
                    accent_text,
                    tokens.accent,
                    0.75,
                    Shadow {
                        offset: Vector::new(0.0, 3.0),
                        blur_radius: 10.0,
                        ..tokens.shadow
                    },
                ),
                button::Status::Pressed => (
                    Some(mix(tokens.accent, Color::BLACK, 0.10)),
                    accent_text,
                    tokens.accent,
                    0.75,
                    Shadow::default(),
                ),
                button::Status::Disabled => (
                    Some(fade(tokens.accent, 0.16)),
                    tokens.disabled,
                    fade(tokens.accent, 0.22),
                    0.75,
                    Shadow::default(),
                ),
            }
        }
        ButtonRole::Segmented { selected } => match (selected, status) {
            (_, button::Status::Disabled) => (
                if selected {
                    Some(fade(tokens.accent, 0.10))
                } else {
                    None
                },
                tokens.disabled,
                transparent,
                0.0,
                Shadow::default(),
            ),
            (true, button::Status::Hovered) => (
                Some(fade(tokens.accent, 0.30)),
                tokens.accent,
                fade(tokens.accent, 0.55),
                0.75,
                Shadow::default(),
            ),
            (true, button::Status::Pressed) => (
                Some(fade(tokens.accent, 0.36)),
                tokens.accent,
                fade(tokens.accent, 0.65),
                0.75,
                Shadow::default(),
            ),
            (true, button::Status::Active) => (
                Some(fade(tokens.accent, 0.23)),
                tokens.accent,
                fade(tokens.accent, 0.48),
                0.75,
                Shadow::default(),
            ),
            (false, button::Status::Hovered) => (
                Some(tokens.raised),
                tokens.text,
                transparent,
                0.0,
                Shadow::default(),
            ),
            (false, button::Status::Pressed) => (
                Some(tokens.input),
                tokens.text,
                transparent,
                0.0,
                Shadow::default(),
            ),
            (false, button::Status::Active) => {
                (None, tokens.text_muted, transparent, 0.0, Shadow::default())
            }
        },
    };

    button::Style {
        background: background.map(Background::Color),
        text_color,
        border: Border {
            color: border_color,
            width: border_width,
            radius: tokens.control_radius.into(),
        },
        shadow,
        ..button::Style::default()
    }
}

/// Produces a closure suitable for [`iced::widget::PickList::style`].
pub(crate) fn pick_list_style(
    tokens: ShellTokens,
) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style + Clone {
    move |_theme, status| {
        let (background, border_color) = match status {
            pick_list::Status::Active => (tokens.input, tokens.border),
            pick_list::Status::Hovered => (tokens.raised, fade(tokens.accent, 0.55)),
            pick_list::Status::Opened { .. } => (tokens.raised, tokens.accent),
        };

        pick_list::Style {
            text_color: tokens.text,
            placeholder_color: tokens.text_muted,
            handle_color: tokens.text_muted,
            background: background.into(),
            border: tokens.control_border(border_color),
        }
    }
}

/// Produces a closure suitable for [`iced::widget::PickList::menu_style`].
pub(crate) fn menu(tokens: ShellTokens) -> impl Fn(&Theme) -> menu::Style + Clone {
    move |_theme| menu::Style {
        background: composite(tokens.glass, tokens.root).into(),
        border: tokens.panel_border(tokens.border),
        text_color: tokens.text,
        selected_text_color: tokens.accent,
        selected_background: fade(tokens.accent, 0.18).into(),
        shadow: tokens.shadow,
    }
}

/// Produces a closure suitable for [`iced::widget::Toggler::style`].
pub(crate) fn toggler_style(
    tokens: ShellTokens,
) -> impl Fn(&Theme, toggler::Status) -> toggler::Style + Clone {
    move |_theme, status| {
        let (is_toggled, is_hovered, is_disabled) = match status {
            toggler::Status::Active { is_toggled } => (is_toggled, false, false),
            toggler::Status::Hovered { is_toggled } => (is_toggled, true, false),
            toggler::Status::Disabled { is_toggled } => (is_toggled, false, true),
        };

        let background = if is_toggled {
            if is_disabled {
                fade(tokens.accent, 0.18)
            } else if is_hovered {
                tone(tokens.accent, tokens.is_dark, 0.10)
            } else {
                tokens.accent
            }
        } else if is_disabled {
            fade(tokens.input, 0.55)
        } else if is_hovered {
            tokens.raised
        } else {
            tokens.input
        };

        let knob = if tokens.is_dark {
            composite(layer(Color::WHITE, 0.84), tokens.root)
        } else {
            Color::WHITE
        };

        toggler::Style {
            background: background.into(),
            background_border_width: 0.75,
            background_border_color: if is_toggled {
                fade(tokens.accent, 0.72)
            } else {
                tokens.border
            },
            foreground: if is_disabled {
                fade(knob, 0.55).into()
            } else {
                knob.into()
            },
            foreground_border_width: 0.5,
            foreground_border_color: tokens.border,
            text_color: Some(if is_disabled {
                tokens.disabled
            } else {
                tokens.text
            }),
            border_radius: None,
            padding_ratio: 0.10,
        }
    }
}

/// Produces a closure suitable for [`iced::widget::Slider::style`].
pub(crate) fn slider_style(
    tokens: ShellTokens,
) -> impl Fn(&Theme, slider::Status) -> slider::Style + Clone {
    move |_theme, status| {
        let (accent, handle_radius) = match status {
            slider::Status::Active => (tokens.accent, 6.5),
            slider::Status::Hovered => (tone(tokens.accent, tokens.is_dark, 0.10), 7.0),
            slider::Status::Dragged => (tone(tokens.accent, tokens.is_dark, 0.16), 7.5),
        };

        slider::Style {
            rail: slider::Rail {
                backgrounds: (accent.into(), tokens.input.into()),
                width: 3.0,
                border: Border {
                    color: tokens.border,
                    width: 0.5,
                    radius: 2.0.into(),
                },
            },
            handle: slider::Handle {
                shape: slider::HandleShape::Circle {
                    radius: handle_radius,
                },
                background: accent.into(),
                border_width: 0.75,
                border_color: fade(tokens.border, 0.85),
            },
        }
    }
}

fn is_dark(color: Rgb) -> bool {
    // Keep this threshold in lockstep with the existing application theme.
    Hsl::from_color(color).lightness < 0.6
}

fn from_rgb(color: Rgb) -> Color {
    Color::from_rgb(color.red, color.green, color.blue)
}

fn from_rgba(color: Rgba) -> Color {
    Color::from_rgba(color.red, color.green, color.blue, color.alpha)
}

fn layer(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn halve_transparency(color: Color) -> Color {
    Color {
        a: 1.0 - (1.0 - color.a) / 2.0,
        ..color
    }
}

fn overlay_glass(tokens: ShellTokens) -> Color {
    if tokens.is_dark {
        layer(
            composite(tokens.glass, tokens.root),
            halve_transparency(tokens.glass).a,
        )
    } else {
        halve_transparency(tokens.glass)
    }
}

fn fade(color: Color, opacity: f32) -> Color {
    Color {
        a: color.a * opacity,
        ..color
    }
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    let inverse = 1.0 - amount;

    Color::from_rgba(
        from.r * inverse + to.r * amount,
        from.g * inverse + to.g * amount,
        from.b * inverse + to.b * amount,
        from.a * inverse + to.a * amount,
    )
}

fn tone(color: Color, is_dark: bool, amount: f32) -> Color {
    mix(
        color,
        if is_dark { Color::WHITE } else { Color::BLACK },
        amount,
    )
}

fn contrast_color(background: Color) -> Color {
    let background = Rgb::new(background.r, background.g, background.b);

    if is_dark(background) {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

fn composite(foreground: Color, background: Color) -> Color {
    let alpha = foreground.a + background.a * (1.0 - foreground.a);

    if alpha <= f32::EPSILON {
        return Color::TRANSPARENT;
    }

    Color::from_rgba(
        (foreground.r * foreground.a + background.r * background.a * (1.0 - foreground.a)) / alpha,
        (foreground.g * foreground.a + background.g * background.a * (1.0 - foreground.a)) / alpha,
        (foreground.b * foreground.a + background.b * background.a * (1.0 - foreground.a)) / alpha,
        alpha,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_same_darkness_threshold_as_the_application_theme() {
        assert!(is_dark(Rgb::new(0.1, 0.1, 0.1)));
        assert!(!is_dark(Rgb::new(0.8, 0.8, 0.8)));
    }

    #[test]
    fn tokens_keep_document_colors_as_shell_semantics() {
        let settings = NfoRenderSettings {
            background_color: Rgb::new(0.05, 0.10, 0.08),
            text_color: Rgba::new(0.9, 0.95, 0.9, 1.0),
            art_color: Rgba::new(0.2, 0.8, 0.55, 1.0),
            ..NfoRenderSettings::default()
        };

        let tokens = ShellTokens::from_settings(&settings);

        assert!(tokens.is_dark);
        assert_eq!(tokens.text, from_rgba(settings.text_color));
        assert_eq!(tokens.accent, from_rgba(settings.art_color));
        assert!(tokens.glass.a < 1.0);
        assert!(tokens.border.a < 1.0);
    }

    #[test]
    fn light_tokens_use_translucent_glass() {
        let tokens = ShellTokens::from_settings(&NfoRenderSettings::default());

        assert!(!tokens.is_dark);
        assert!(tokens.toolbar.a < 1.0);
        assert!(tokens.glass.a < 1.0);
        assert!(tokens.raised.a < 1.0);
    }

    #[test]
    fn modal_glass_is_opaque_in_dark_and_light_themes() {
        let dark = NfoRenderSettings {
            background_color: Rgb::new(0.01, 0.02, 0.02),
            ..NfoRenderSettings::default()
        };
        let light = NfoRenderSettings::default();

        for settings in [&dark, &light] {
            let tokens = ShellTokens::from_settings(settings);
            let style = surface_appearance(tokens, SurfaceRole::ModalGlass);
            let Some(Background::Gradient(gradient::Gradient::Linear(gradient))) = style.background
            else {
                panic!("modal glass must use a gradient");
            };

            assert!(
                gradient
                    .stops
                    .iter()
                    .flatten()
                    .all(|stop| stop.color.a == 1.0)
            );
        }
    }

    #[test]
    fn navigator_and_overflow_menu_use_half_transparency_glass() {
        let dark = ShellTokens::from_settings(&NfoRenderSettings {
            background_color: Rgb::new(0.01, 0.02, 0.02),
            ..NfoRenderSettings::default()
        });
        let light = ShellTokens::from_settings(&NfoRenderSettings::default());

        for tokens in [dark, light] {
            let navigator = surface_appearance(tokens, SurfaceRole::NavigatorGlass);
            let overflow = surface_appearance(tokens, SurfaceRole::Glass);

            assert_eq!(navigator.background, overflow.background);
            assert_eq!(navigator.border, overflow.border);
            assert_eq!(navigator.shadow.blur_radius, 30.0);

            let Some(Background::Color(background)) = navigator.background else {
                panic!("navigator glass must use a solid background");
            };
            assert_eq!(background, overlay_glass(tokens));
            assert_eq!(background.a, 1.0 - (1.0 - tokens.glass.a) / 2.0);
        }
    }

    #[test]
    fn backdrop_surfaces_use_the_specified_dark_opacities() {
        let tokens = ShellTokens::from_settings(&NfoRenderSettings {
            background_color: Rgb::new(0.01, 0.02, 0.02),
            ..NfoRenderSettings::default()
        });

        assert_surface_alpha(tokens, SurfaceRole::BackdropCanvas, 0.78);
        assert_surface_alpha(tokens, SurfaceRole::BackdropToolbar, 0.86);
        assert_inspector_gradient(tokens, 0.64);
    }

    #[test]
    fn backdrop_surfaces_use_the_specified_light_opacities() {
        let tokens = ShellTokens::from_settings(&NfoRenderSettings::default());

        assert_surface_alpha(tokens, SurfaceRole::BackdropCanvas, 0.80);
        assert_surface_alpha(tokens, SurfaceRole::BackdropToolbar, 0.92);
        assert_inspector_gradient(tokens, 0.82);
    }

    fn assert_surface_alpha(tokens: ShellTokens, role: SurfaceRole, expected_alpha: f32) {
        let style = surface_appearance(tokens, role);
        let Some(Background::Color(background)) = style.background else {
            panic!("backdrop canvas and toolbar must use color backgrounds");
        };

        assert_eq!(background.a, expected_alpha);
    }

    fn assert_inspector_gradient(tokens: ShellTokens, expected_alpha: f32) {
        let style = surface_appearance(tokens, SurfaceRole::BackdropInspector);
        let Some(Background::Gradient(gradient::Gradient::Linear(gradient))) = style.background
        else {
            panic!("backdrop inspector must use a linear gradient");
        };
        let stops = gradient.stops.iter().flatten().collect::<Vec<_>>();

        assert_eq!(stops.len(), 3);
        assert_eq!(stops[0].offset, 0.0);
        assert_eq!(stops[1].offset, 0.52);
        assert_eq!(stops[2].offset, 1.0);
        assert!(stops.iter().all(|stop| stop.color.a == expected_alpha));
        assert_eq!(stops[0].color, stops[1].color);
        assert_ne!(stops[1].color, stops[2].color);
        assert_eq!(style.border.width, 0.75);
        assert_eq!(style.shadow.offset, Vector::new(-3.0, 0.0));
        assert_eq!(style.shadow.blur_radius, 18.0);
    }
}
