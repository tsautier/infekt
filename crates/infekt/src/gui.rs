pub mod about_screen;
pub mod main_view;
#[path = "gui/preferences/named_colors.rs"]
pub(crate) mod named_colors;
pub(crate) mod presentation_inspector;
pub(crate) mod shell_style;
mod utils;
mod widget;

pub(crate) use widget::adjacent_pair::AdjacentPair;
pub(crate) use widget::anchored_overlay::AnchoredOverlay;
