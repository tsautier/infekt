use std::sync::Arc;

use colornames::Color as NamedColor;
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    column, container, pick_list, row, rule, scrollable, slider, space, text, toggler,
};
use iced::{Alignment, Element, Length, Task};

use crate::core::nfo_data::NfoData;
use crate::presentation::{BUILT_IN_THEMES, NfoThemePreset, PresentationState};
use crate::settings::NfoRenderSettings;

use super::named_colors;
use super::shell_style::{self, ShellTokens, SurfaceRole};

const INSPECTOR_WIDTH: f32 = 320.0;
const FONT_SIZES: &[f32] = &[8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0];

#[derive(Debug, Clone)]
pub(crate) enum Message {
    ThemeSelected(NfoThemePreset),
    BackgroundColorSelected(NamedColor),
    TextColorSelected(NamedColor),
    ArtColorSelected(NamedColor),
    HyperlinkColorSelected(NamedColor),
    GlowEnabledChanged(bool),
    GlowColorSelected(NamedColor),
    GlowRadiusChanged(u16),
    HyperlinkUnderlineChanged(bool),
    FontNamesLoaded(Vec<String>),
    FontNameSelected(String),
    FontSizeSelected(f32),
    CharacterRatioChanged(f32),
    AntialiasingChanged(bool),
    UseAnsiColorsChanged(bool),
    LineWrappingChanged(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Update {
    PresentationOnly,
    RenderSettingsChanged,
}

#[derive(Default)]
pub(crate) struct PresentationInspector {
    all_font_names: Vec<String>,
}

impl PresentationInspector {
    pub(crate) fn load_font_names() -> Task<Message> {
        Task::perform(
            async {
                let mut database = fontdb::Database::new();
                database.load_system_fonts();

                let mut names = database
                    .faces()
                    .filter(|face| face.monospaced)
                    .filter(|face| !face.families.is_empty())
                    .map(|face| face.families[0].0.to_string())
                    .filter(|name| !name.is_empty() && !name.starts_with('.'))
                    .collect::<Vec<_>>();

                names.extend([
                    "Andale Mono".to_owned(),
                    "Cascadia Mono".to_owned(),
                    "Fira Mono".to_owned(),
                    "Menlo Nerd Font Mono".to_owned(),
                ]);
                names.sort();
                names.dedup();
                names
            },
            Message::FontNamesLoaded,
        )
    }

    pub(crate) fn update(
        &mut self,
        message: Message,
        presentation: &mut PresentationState,
        settings: &mut NfoRenderSettings,
    ) -> Update {
        match message {
            Message::ThemeSelected(preset) => {
                presentation.select_theme(preset, settings);
                Update::RenderSettingsChanged
            }
            Message::BackgroundColorSelected(color) => {
                let color = named_colors::to_palette_rgb(color);
                settings.background_color = color;
                presentation.theme_values.background_color = color;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::TextColorSelected(color) => {
                let color = named_colors::to_palette_rgba(color);
                settings.text_color = color;
                presentation.theme_values.text_color = color;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::ArtColorSelected(color) => {
                let color = named_colors::to_palette_rgba(color);
                settings.art_color = color;
                presentation.theme_values.art_color = color;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::HyperlinkColorSelected(color) => {
                let color = named_colors::to_palette_rgba(color);
                settings.hyperlink_color = color;
                presentation.theme_values.hyperlink_color = color;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::GlowEnabledChanged(enabled) => {
                settings.blur_enabled = enabled;
                presentation.theme_values.glow_enabled = enabled;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::GlowColorSelected(color) => {
                let color = named_colors::to_palette_rgba(color);
                settings.blur_color = color;
                presentation.theme_values.glow_color = color;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::GlowRadiusChanged(radius) => {
                settings.blur_radius = radius;
                presentation.theme_values.glow_radius = radius;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::HyperlinkUnderlineChanged(underline) => {
                settings.hyperlink_underline = underline;
                presentation.theme_values.hyperlink_underline = underline;
                presentation.mark_custom();
                Update::RenderSettingsChanged
            }
            Message::FontNamesLoaded(names) => {
                self.all_font_names = names;
                Update::PresentationOnly
            }
            Message::FontNameSelected(name) => {
                settings.font_name = name;
                Update::RenderSettingsChanged
            }
            Message::FontSizeSelected(size) => {
                settings.classic_font_size = size;
                Update::RenderSettingsChanged
            }
            Message::CharacterRatioChanged(ratio) => {
                presentation.character_ratio = ratio;
                settings.enhanced_view_block_width =
                    (f32::from(settings.enhanced_view_block_height) * ratio)
                        .round()
                        .max(1.0) as u16;
                Update::RenderSettingsChanged
            }
            Message::AntialiasingChanged(enabled) => {
                presentation.antialiasing = enabled;
                Update::PresentationOnly
            }
            Message::UseAnsiColorsChanged(enabled) => {
                presentation.use_ansi_colors = enabled;
                Update::PresentationOnly
            }
            Message::LineWrappingChanged(enabled) => {
                presentation.line_wrapping = enabled;
                Update::PresentationOnly
            }
        }
    }

    pub(crate) fn view<'a>(
        &'a self,
        presentation: &'a PresentationState,
        settings: &'a Arc<NfoRenderSettings>,
        current_nfo: &'a NfoData,
    ) -> Element<'a, Message> {
        let content = column![
            self.header(),
            self.theme_section(presentation, settings),
            divider(),
            self.typography_section(presentation, settings),
            divider(),
            self.display_section(presentation, settings, current_nfo),
            divider(),
            self.properties_section(current_nfo),
        ]
        .spacing(12)
        .padding(18)
        .width(Length::Fill);

        container(
            scrollable(content)
                .direction(Direction::Vertical(
                    Scrollbar::default().width(3).scroller_width(3).margin(1),
                ))
                .height(Length::Fill),
        )
        .width(INSPECTOR_WIDTH)
        .height(Length::Fill)
        .into()
    }

    fn header(&self) -> Element<'_, Message> {
        row![text("Presentation").size(20), space::horizontal()]
            .align_y(Alignment::Center)
            .into()
    }

    fn theme_section<'a>(
        &'a self,
        presentation: &'a PresentationState,
        settings: &'a NfoRenderSettings,
    ) -> Element<'a, Message> {
        let tokens = ShellTokens::from_settings(settings);
        let theme_picker = pick_list(
            BUILT_IN_THEMES.as_slice(),
            Some(presentation.selected_theme),
            Message::ThemeSelected,
        )
        .width(Length::Fill)
        .text_size(15)
        .style(shell_style::pick_list_style(tokens))
        .menu_style(shell_style::menu(tokens));

        column![
            section_label("Theme"),
            theme_picker,
            row![
                color_row(
                    "Background",
                    named_colors::from_palette_rgb(settings.background_color),
                    settings.background_color.into_components(),
                    Message::BackgroundColorSelected,
                    tokens,
                ),
                color_row_rgba(
                    "Text",
                    named_colors::from_palette_rgba(settings.text_color),
                    settings.text_color,
                    Message::TextColorSelected,
                    tokens,
                ),
            ]
            .spacing(12),
            row![
                color_row_rgba(
                    "Art",
                    named_colors::from_palette_rgba(settings.art_color),
                    settings.art_color,
                    Message::ArtColorSelected,
                    tokens,
                ),
                color_row_rgba(
                    "Links",
                    named_colors::from_palette_rgba(settings.hyperlink_color),
                    settings.hyperlink_color,
                    Message::HyperlinkColorSelected,
                    tokens,
                ),
            ]
            .spacing(12),
            toggle_row(
                "Glow",
                settings.blur_enabled,
                Message::GlowEnabledChanged,
                tokens,
            ),
            color_row_rgba(
                "Glow Color",
                named_colors::from_palette_rgba(settings.blur_color),
                settings.blur_color,
                Message::GlowColorSelected,
                tokens,
            ),
            row![
                text("Glow Radius").size(14).width(Length::Fill),
                slider(0..=48, settings.blur_radius, Message::GlowRadiusChanged)
                    .step(1_u16)
                    .width(110)
                    .style(shell_style::slider_style(tokens)),
                text(settings.blur_radius).size(13).width(30),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            toggle_row(
                "Underline Links",
                settings.hyperlink_underline,
                Message::HyperlinkUnderlineChanged,
                tokens,
            ),
        ]
        .spacing(10)
        .into()
    }

    fn typography_section<'a>(
        &'a self,
        presentation: &'a PresentationState,
        settings: &'a NfoRenderSettings,
    ) -> Element<'a, Message> {
        let tokens = ShellTokens::from_settings(settings);
        let font_picker = pick_list(
            self.all_font_names.clone(),
            Some(settings.font_name.clone()),
            Message::FontNameSelected,
        )
        .width(Length::FillPortion(3))
        .text_size(14)
        .menu_height(320)
        .style(shell_style::pick_list_style(tokens))
        .menu_style(shell_style::menu(tokens));

        let size_picker = pick_list(
            FONT_SIZES,
            Some(settings.classic_font_size),
            Message::FontSizeSelected,
        )
        .width(80)
        .text_size(14)
        .style(shell_style::pick_list_style(tokens))
        .menu_style(shell_style::menu(tokens));

        column![
            section_label("Typography"),
            control_row("Font", font_picker.into()),
            control_row("Size", size_picker.into()),
            row![
                text("Character Ratio").size(14).width(Length::Fill),
                slider(
                    0.4..=0.8,
                    presentation.character_ratio,
                    Message::CharacterRatioChanged,
                )
                .step(0.01_f32)
                .width(100)
                .style(shell_style::slider_style(tokens)),
                text(format!("{:.2}", presentation.character_ratio))
                    .size(13)
                    .width(34),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            toggle_row(
                "Anti-aliasing",
                presentation.antialiasing,
                Message::AntialiasingChanged,
                tokens,
            ),
        ]
        .spacing(10)
        .into()
    }

    fn display_section<'a>(
        &'a self,
        presentation: &'a PresentationState,
        settings: &'a NfoRenderSettings,
        current_nfo: &'a NfoData,
    ) -> Element<'a, Message> {
        let tokens = ShellTokens::from_settings(settings);
        let encoding = if current_nfo.is_loaded() {
            current_nfo.get_charset_name()
        } else {
            "No file loaded"
        };

        column![
            section_label("Display Options"),
            container(text(encoding).size(14))
                .padding([7, 9])
                .width(Length::Fill)
                .style(shell_style::surface(tokens, SurfaceRole::Input)),
            toggle_row(
                "Use ANSI colors",
                presentation.use_ansi_colors,
                Message::UseAnsiColorsChanged,
                tokens,
            ),
            toggle_row(
                "Line wrapping",
                presentation.line_wrapping,
                Message::LineWrappingChanged,
                tokens,
            ),
        ]
        .spacing(10)
        .into()
    }

    fn properties_section<'a>(&'a self, current_nfo: &'a NfoData) -> Element<'a, Message> {
        let mut properties = column![section_label("Properties")].spacing(5);

        if current_nfo.is_loaded() {
            properties = properties
                .push(property_row(
                    "Filename",
                    current_nfo.get_file_name().unwrap_or_default(),
                ))
                .push(property_row(
                    "Path",
                    current_nfo
                        .get_file_path()
                        .map(|path| path.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ));

            if let Some(grid) = current_nfo.get_renderer_grid() {
                properties = properties.push(property_row(
                    "Dimensions",
                    format!("{} × {}", grid.width, grid.height),
                ));
            }

            properties = properties.push(property_row("Encoding", current_nfo.get_charset_name()));
        } else {
            properties = properties.push(text("Open an NFO to see file details.").size(12));
        }

        properties.into()
    }
}

fn section_label(label: &'static str) -> Element<'static, Message> {
    text(label).size(13).into()
}

fn divider() -> Element<'static, Message> {
    rule::horizontal(1).into()
}

fn toggle_row(
    label: &'static str,
    value: bool,
    message: fn(bool) -> Message,
    tokens: ShellTokens,
) -> Element<'static, Message> {
    row![
        text(label).size(14).width(Length::Fill),
        toggler(value)
            .on_toggle(message)
            .size(20)
            .style(shell_style::toggler_style(tokens)),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn control_row<'a>(label: &'static str, control: Element<'a, Message>) -> Element<'a, Message> {
    row![text(label).size(14).width(Length::FillPortion(2)), control]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
}

fn color_row(
    label: &'static str,
    selected: Option<NamedColor>,
    rgb: (f32, f32, f32),
    message: fn(NamedColor) -> Message,
    tokens: ShellTokens,
) -> Element<'static, Message> {
    color_control(
        label,
        selected,
        iced::Color::from_rgb(rgb.0, rgb.1, rgb.2),
        message,
        tokens,
    )
}

fn color_row_rgba(
    label: &'static str,
    selected: Option<NamedColor>,
    rgba: palette::rgb::Rgba,
    message: fn(NamedColor) -> Message,
    tokens: ShellTokens,
) -> Element<'static, Message> {
    color_control(
        label,
        selected,
        iced::Color::from_rgba(rgba.red, rgba.green, rgba.blue, rgba.alpha),
        message,
        tokens,
    )
}

fn color_control(
    label: &'static str,
    selected: Option<NamedColor>,
    swatch_color: iced::Color,
    message: fn(NamedColor) -> Message,
    tokens: ShellTokens,
) -> Element<'static, Message> {
    let swatch_text =
        if swatch_color.r * 0.299 + swatch_color.g * 0.587 + swatch_color.b * 0.114 > 0.55 {
            iced::Color::from_rgb(0.10, 0.10, 0.11)
        } else {
            iced::Color::WHITE
        };

    let picker = pick_list(named_colors::ALL, selected, message)
        .placeholder("      ")
        .width(58)
        .text_size(13)
        .menu_height(300)
        .style(move |_theme, status| iced::widget::pick_list::Style {
            text_color: swatch_color,
            placeholder_color: swatch_color,
            handle_color: swatch_text,
            background: swatch_color.into(),
            border: iced::Border {
                color: match status {
                    iced::widget::pick_list::Status::Active => tokens.border,
                    iced::widget::pick_list::Status::Hovered => tokens.accent,
                    iced::widget::pick_list::Status::Opened { .. } => tokens.accent,
                },
                width: 1.0,
                radius: 6.0.into(),
            },
        })
        .menu_style(shell_style::menu(tokens));

    row![text(label).size(13).width(Length::Fill), picker,]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .into()
}

fn property_row(label: &'static str, value: impl ToString) -> Element<'static, Message> {
    row![
        text(format!("{label}:")).size(12).width(72),
        text(value.to_string()).size(12).width(Length::Fill),
    ]
    .spacing(5)
    .align_y(Alignment::Start)
    .into()
}
