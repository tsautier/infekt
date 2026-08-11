use iced::gradient;
use iced::widget::{button, column, container, opaque, row, space, stack, svg, text};
use iced::{Alignment, Background, Color, Element, Length, Theme};

use crate::gui::main_view::{self, TabId};
use crate::gui::shell_style::{self, ButtonRole, ShellTokens, SurfaceRole};
use crate::gui::{AdjacentPair, AnchoredOverlay, BackdropParallax};

use super::file_drop::FileDropHover;
use super::{InfektApp, Message};

const TOOLBAR_HEIGHT: f32 = 62.0;
const INSPECTOR_WIDTH: f32 = 320.0;

const OPEN_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/folder-open.svg");
const FILE_ICON: &[u8] = include_bytes!("../../../../third_party/tabler-icons/outline/file.svg");
const ZOOM_OUT_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/minus.svg");
const ZOOM_IN_ICON: &[u8] = include_bytes!("../../../../third_party/tabler-icons/outline/plus.svg");
const INSPECTOR_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/layout-sidebar-right.svg");
const EXPORT_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/upload.svg");
const MORE_ICON: &[u8] = include_bytes!("../../../../third_party/tabler-icons/outline/dots.svg");
const CLOSE_ICON: &[u8] = include_bytes!("../../../../third_party/tabler-icons/outline/x.svg");
const PREVIOUS_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/chevron-left.svg");
const NEXT_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/chevron-right.svg");
const DROP_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/drag-drop.svg");
const DROP_WARNING_ICON: &[u8] =
    include_bytes!("../../../../third_party/tabler-icons/outline/alert-triangle.svg");

impl InfektApp {
    pub fn view(&self) -> Element<'_, Message> {
        let tokens = ShellTokens::from(&self.active_render_settings);
        let backdrop = self.backdrop.current_handle().cloned();
        let has_backdrop = backdrop.is_some();
        let toolbar = self.toolbar(tokens, has_backdrop);
        let viewer = container(
            self.main_view
                .view(&self.current_nfo)
                .map(Message::MainView),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(shell_style::surface(
            tokens,
            if has_backdrop {
                SurfaceRole::BackdropCanvas
            } else {
                SurfaceRole::Canvas
            },
        ));
        let viewer: Element<'_, Message> =
            if let Some((position, total)) = self.folder_browser.position() {
                stack![viewer, self.folder_navigator(tokens, position, total)]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into()
            } else {
                viewer.into()
            };

        let content: Element<'_, Message> = if self.presentation.inspector_open {
            let inspector = self
                .presentation_inspector
                .view(
                    &self.presentation,
                    &self.active_render_settings,
                    &self.current_nfo,
                )
                .map(Message::Inspector);
            let inspector = container(inspector)
                .width(INSPECTOR_WIDTH)
                .height(Length::Fill)
                .style(shell_style::surface(
                    tokens,
                    if has_backdrop {
                        SurfaceRole::BackdropInspector
                    } else {
                        SurfaceRole::Inspector
                    },
                ));

            row![viewer, inspector].height(Length::Fill).into()
        } else {
            viewer
        };

        let base = container(space::horizontal())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| root_style(tokens));
        let shell = container(column![toolbar, content].height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style::default().color(tokens.text));

        let mut root = stack![base].width(Length::Fill).height(Length::Fill);

        if let Some(handle) = backdrop {
            root = root.push(BackdropParallax::new(
                handle,
                self.main_view.backdrop_translation(),
                if tokens.is_dark { 0.82_f32 } else { 0.76_f32 },
                main_view::BACKDROP_PARALLAX_LIMIT,
            ));
        }

        root = root.push(shell);

        let mut layers = stack![root].width(Length::Fill).height(Length::Fill);

        if self.presentation.about_open {
            layers = layers.push(self.about_overlay(tokens));
        }

        if let Some(hover) = self.file_drop.hover() {
            layers = layers.push(self.file_drop_overlay(tokens, hover));
        }

        layers.into()
    }

    fn toolbar(&self, tokens: ShellTokens, has_backdrop: bool) -> Element<'_, Message> {
        container(self.toolbar_contents(tokens))
            .padding([8, 12])
            .height(TOOLBAR_HEIGHT)
            .width(Length::Fill)
            .style(shell_style::surface(
                tokens,
                if has_backdrop {
                    SurfaceRole::BackdropToolbar
                } else {
                    SurfaceRole::Toolbar
                },
            ))
            .into()
    }

    fn toolbar_contents(&self, tokens: ShellTokens) -> Element<'_, Message> {
        let open_button = button(
            row![icon(OPEN_ICON, tokens.text, 18.0), text("Open…").size(13)]
                .spacing(7)
                .align_y(Alignment::Center),
        )
        .padding([7, 10])
        .on_press(Message::OpenFileDialog)
        .style(shell_style::button_style(tokens, ButtonRole::Glass));

        let modes = self.mode_selector(tokens);
        let zoom = self.zoom_control(tokens);

        let inspector_button = button(icon(INSPECTOR_ICON, tokens.text, 18.0))
            .padding(8)
            .on_press(Message::ToggleInspector)
            .style(shell_style::button_style(
                tokens,
                ButtonRole::Segmented {
                    selected: self.presentation.inspector_open,
                },
            ));

        let export_button = button(
            row![
                icon(EXPORT_ICON, tokens.disabled, 17.0),
                text("Export").size(13).color(tokens.disabled)
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([7, 10])
        .style(shell_style::button_style(tokens, ButtonRole::Glass));

        let more_button = button(icon(MORE_ICON, tokens.text, 18.0))
            .padding(8)
            .on_press(Message::ToggleOverflow)
            .style(shell_style::button_style(
                tokens,
                ButtonRole::Segmented {
                    selected: self.presentation.overflow_open,
                },
            ));
        let more_button = AnchoredOverlay::new(
            more_button,
            self.overflow_menu(tokens),
            self.presentation.overflow_open,
        )
        .gap(20.0);

        let left = row![open_button, self.file_information(tokens)]
            .spacing(9)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .clip(true);
        let center = row![modes, zoom].spacing(9).align_y(Alignment::Center);
        let right = row![
            space::horizontal(),
            export_button,
            more_button,
            inspector_button,
        ]
        .spacing(9)
        .align_y(Alignment::Center)
        .width(Length::Fill);

        row![
            container(left).width(Length::FillPortion(1)).clip(true),
            center,
            container(right).width(Length::FillPortion(1)),
        ]
        .spacing(9)
        .align_y(Alignment::Center)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn file_information(&self, tokens: ShellTokens) -> Element<'_, Message> {
        if !self.current_nfo.is_loaded() {
            return row![
                icon(FILE_ICON, tokens.text_muted, 17.0),
                text("No file open").size(13).color(tokens.text_muted),
            ]
            .spacing(7)
            .align_y(Alignment::Center)
            .width(Length::Fill)
            .into();
        }

        let metadata = self.current_nfo.get_renderer_grid().map_or_else(
            || self.current_nfo.get_charset_name().to_owned(),
            |grid| {
                format!(
                    "{}×{} · {}",
                    grid.width,
                    grid.height,
                    self.current_nfo.get_charset_name()
                )
            },
        );

        let filename = container(
            text(self.current_nfo.get_file_name().unwrap_or_default())
                .size(13)
                .wrapping(text::Wrapping::None),
        )
        .clip(true);
        let metadata = text(format!("— {metadata}"))
            .size(11)
            .color(tokens.text_muted)
            .wrapping(text::Wrapping::None);

        row![
            icon(FILE_ICON, tokens.text, 17.0),
            container(AdjacentPair::new(filename, metadata).spacing(7.0))
                .width(Length::Fill)
                .clip(true),
        ]
        .spacing(7)
        .align_y(Alignment::Center)
        .width(Length::Fill)
        .clip(true)
        .into()
    }

    fn mode_selector(&self, tokens: ShellTokens) -> Element<'_, Message> {
        let mode = |label: &'static str, tab: TabId| {
            let selected = self.main_view.active_tab() == tab;
            let mut control = button(text(label).size(13).center())
                .padding([7, 11])
                .style(shell_style::button_style(
                    tokens,
                    ButtonRole::Segmented { selected },
                ));

            if self.current_nfo.is_loaded() {
                control = control.on_press(Message::MainView(main_view::Message::TabSelected(tab)));
            }

            control
        };

        container(
            row![
                mode("Enhanced", TabId::Enhanced),
                mode("Classic", TabId::Classic),
                mode("Text Only", TabId::TextOnly),
            ]
            .spacing(1),
        )
        .padding(2)
        .style(shell_style::surface(tokens, SurfaceRole::Input))
        .into()
    }

    fn zoom_control(&self, tokens: ShellTokens) -> Element<'_, Message> {
        let minus = button(icon(ZOOM_OUT_ICON, tokens.text, 16.0))
            .padding(6)
            .on_press(Message::ZoomOut)
            .style(shell_style::button_style(tokens, ButtonRole::Toolbar));
        let plus = button(icon(ZOOM_IN_ICON, tokens.text, 16.0))
            .padding(6)
            .on_press(Message::ZoomIn)
            .style(shell_style::button_style(tokens, ButtonRole::Toolbar));

        container(
            row![
                minus,
                text(format!("{}%", self.presentation.zoom_percent))
                    .size(12)
                    .center()
                    .width(47),
                plus,
            ]
            .spacing(1)
            .align_y(Alignment::Center),
        )
        .padding(2)
        .style(shell_style::surface(tokens, SurfaceRole::Input))
        .into()
    }

    fn overflow_menu(&self, tokens: ShellTokens) -> Element<'_, Message> {
        let about = button(text("About iNFekt").size(13))
            .width(Length::Fill)
            .padding([8, 10])
            .on_press(Message::ShowAbout)
            .style(shell_style::button_style(tokens, ButtonRole::Toolbar));

        container(column![about].width(Length::Fill))
            .padding(6)
            .width(170)
            .style(shell_style::surface(tokens, SurfaceRole::Glass))
            .into()
    }

    fn folder_navigator(
        &self,
        tokens: ShellTokens,
        position: usize,
        total: usize,
    ) -> Element<'_, Message> {
        let previous = button(icon(PREVIOUS_ICON, tokens.text, 16.0))
            .width(30)
            .height(30)
            .padding(6)
            .on_press(Message::Browse(
                super::folder_browser::BrowseDirection::Previous,
            ))
            .style(shell_style::button_style(tokens, ButtonRole::Toolbar));
        let next = button(icon(NEXT_ICON, tokens.text, 16.0))
            .width(30)
            .height(30)
            .padding(6)
            .on_press(Message::Browse(
                super::folder_browser::BrowseDirection::Next,
            ))
            .style(shell_style::button_style(tokens, ButtonRole::Toolbar));
        let position = text(format!("{position} of {total}"))
            .size(12)
            .center()
            .width(68);
        let navigator = container(
            row![previous, position, next]
                .spacing(3)
                .align_y(Alignment::Center),
        )
        .padding(4)
        .style(shell_style::surface(tokens, SurfaceRole::NavigatorGlass));

        container(navigator)
            .center_x(Length::Fill)
            .align_bottom(Length::Fill)
            .padding(iced::Padding {
                top: 0.0,
                right: 0.0,
                bottom: 24.0,
                left: 0.0,
            })
            .into()
    }

    fn about_overlay(&self, tokens: ShellTokens) -> Element<'_, Message> {
        let close = button(icon(CLOSE_ICON, tokens.text, 17.0))
            .padding(7)
            .on_press(Message::CloseAbout)
            .style(shell_style::button_style(tokens, ButtonRole::Toolbar));

        let modal = container(
            column![
                row![text("About").size(17), space::horizontal(), close].align_y(Alignment::Center),
                self.about_screen.view().map(Message::About),
            ]
            .spacing(4),
        )
        .padding(16)
        .width(720)
        .height(410)
        .style(shell_style::surface(tokens, SurfaceRole::ModalGlass));

        opaque(
            container(modal)
                .center(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| {
                    container::Style::default().background(Color::from_rgba(0.0, 0.0, 0.0, 0.34))
                }),
        )
    }

    fn file_drop_overlay(
        &self,
        tokens: ShellTokens,
        hover: FileDropHover<'_>,
    ) -> Element<'_, Message> {
        let (message, cue_icon, cue_color) = match hover {
            FileDropHover::Single(path) => {
                let filename = path
                    .file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy();

                (format!("Drop to open {filename}"), DROP_ICON, tokens.accent)
            }
            FileDropHover::Multiple(count) => (
                format!("{count} files selected — drop exactly one file"),
                DROP_WARNING_ICON,
                self.theme
                    .as_ref()
                    .map_or(tokens.secondary_accent, |theme| theme.palette().warning),
            ),
        };
        let cue = container(
            column![
                icon(cue_icon, cue_color, 44.0),
                text(message)
                    .size(20)
                    .color(tokens.text)
                    .center()
                    .width(Length::Fill)
                    .wrapping(text::Wrapping::WordOrGlyph),
            ]
            .spacing(16)
            .align_x(Alignment::Center),
        )
        .padding([28, 36])
        .width(Length::Fill)
        .max_width(560)
        .style(shell_style::surface(tokens, SurfaceRole::ModalGlass));

        opaque(
            container(cue)
                .padding(24)
                .center(Length::Fill)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| {
                    container::Style::default().background(Color::from_rgba(0.0, 0.0, 0.0, 0.34))
                }),
        )
    }
}

fn icon<'a>(bytes: &'static [u8], color: Color, size: f32) -> svg::Svg<'a> {
    svg(svg::Handle::from_memory(bytes))
        .width(size)
        .height(size)
        .style(move |_theme: &Theme, _status| svg::Style { color: Some(color) })
}

fn root_style(tokens: ShellTokens) -> container::Style {
    let wash = mix(
        tokens.root,
        tokens.accent,
        if tokens.is_dark { 0.06 } else { 0.12 },
    );
    let gradient = gradient::Linear::new(std::f32::consts::FRAC_PI_2)
        .add_stop(0.0, tokens.root)
        .add_stop(0.58, tokens.root)
        .add_stop(1.0, wash);

    container::Style::default()
        .color(tokens.text)
        .background(Background::Gradient(gradient.into()))
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
