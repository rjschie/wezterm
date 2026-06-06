use crate::*;
use std::convert::TryFrom;
use wezterm_dynamic::{FromDynamic, ToDynamic};

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct FancyBarBoxDimension {
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub top: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub bottom: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub left: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub right: Option<Dimension>,
}

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct FancyBarCornerRounding {
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub top_left: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub top_right: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub bottom_left: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub bottom_right: Option<Dimension>,
}

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct FancyBarBoxColor {
    pub top: Option<RgbaColor>,
    pub bottom: Option<RgbaColor>,
    pub left: Option<RgbaColor>,
    pub right: Option<RgbaColor>,
}

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct FancyBarTabStyle {
    pub bg_color: Option<RgbaColor>,
    pub fg_color: Option<RgbaColor>,
    pub hover_bg_color: Option<RgbaColor>,
    pub hover_fg_color: Option<RgbaColor>,
    #[dynamic(default)]
    pub padding: Option<FancyBarBoxDimension>,
    #[dynamic(default)]
    pub margin: Option<FancyBarBoxDimension>,
    #[dynamic(default)]
    pub rounding: Option<FancyBarCornerRounding>,
    #[dynamic(default)]
    pub border: Option<FancyBarBoxDimension>,
    #[dynamic(default)]
    pub border_color: Option<FancyBarBoxColor>,
}

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct FancyBarButtonStyle {
    pub bg_color: Option<RgbaColor>,
    pub fg_color: Option<RgbaColor>,
    pub hover_bg_color: Option<RgbaColor>,
    pub hover_fg_color: Option<RgbaColor>,
    #[dynamic(default)]
    pub text: Option<String>,
    #[dynamic(default)]
    pub font_size: Option<f64>,
    #[dynamic(default)]
    pub margin: Option<FancyBarBoxDimension>,
    #[dynamic(default)]
    pub padding: Option<FancyBarBoxDimension>,
    #[dynamic(default)]
    pub rounding: Option<FancyBarCornerRounding>,
    #[dynamic(default)]
    pub border: Option<FancyBarBoxDimension>,
    #[dynamic(default)]
    pub border_color: Option<FancyBarBoxColor>,
}

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct FancyBarSeparatorStyle {
    pub color: Option<RgbaColor>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub thickness: Option<Dimension>,
    #[dynamic(default)]
    pub margin: Option<FancyBarBoxDimension>,
}

#[derive(Default, Debug, Clone, FromDynamic, ToDynamic)]
pub struct FancyBarConfig {
    #[dynamic(default)]
    pub font: Option<TextStyle>,
    #[dynamic(default)]
    pub font_size: Option<f64>,
    pub background: Option<RgbaColor>,
    pub inactive_background: Option<RgbaColor>,
    #[dynamic(default)]
    pub padding: Option<FancyBarBoxDimension>,
    #[dynamic(default)]
    pub use_full_width: Option<bool>,
    #[dynamic(default)]
    pub active_tab: Option<FancyBarTabStyle>,
    #[dynamic(default)]
    pub inactive_tab: Option<FancyBarTabStyle>,
    #[dynamic(default)]
    pub tab_separator: Option<FancyBarSeparatorStyle>,
    #[dynamic(default)]
    pub active_close_tab_button: Option<FancyBarButtonStyle>,
    #[dynamic(default)]
    pub inactive_close_tab_button: Option<FancyBarButtonStyle>,
    #[dynamic(default)]
    pub new_tab_button: Option<FancyBarButtonStyle>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub tab_min_width: Option<Dimension>,
    #[dynamic(try_from = "crate::units::OptPixelUnit", default)]
    pub tab_max_width: Option<Dimension>,
    #[dynamic(default)]
    pub tab_text_align: FancyBarTextAlign,
    #[dynamic(default)]
    pub tab_placement: FancyBarTabPlacement,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, FromDynamic, ToDynamic)]
#[dynamic(try_from = "String", into = "String")]
pub enum FancyBarTextAlign {
    Left,
    #[default]
    Center,
    Right,
}

impl From<&FancyBarTextAlign> for String {
    fn from(val: &FancyBarTextAlign) -> String {
        match val {
            FancyBarTextAlign::Left => "Left".to_string(),
            FancyBarTextAlign::Center => "Center".to_string(),
            FancyBarTextAlign::Right => "Right".to_string(),
        }
    }
}

impl TryFrom<String> for FancyBarTextAlign {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.eq_ignore_ascii_case("left") {
            Ok(Self::Left)
        } else if value.eq_ignore_ascii_case("center") {
            Ok(Self::Center)
        } else if value.eq_ignore_ascii_case("right") {
            Ok(Self::Right)
        } else {
            Err(format!(
                "invalid text_align '{}', expected 'left', 'center', or 'right'",
                value
            ))
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, FromDynamic, ToDynamic)]
#[dynamic(try_from = "String", into = "String")]
pub enum FancyBarTabPlacement {
    #[default]
    Left,
    Center,
    Right,
}

impl From<&FancyBarTabPlacement> for String {
    fn from(val: &FancyBarTabPlacement) -> String {
        match val {
            FancyBarTabPlacement::Left => "Left".to_string(),
            FancyBarTabPlacement::Center => "Center".to_string(),
            FancyBarTabPlacement::Right => "Right".to_string(),
        }
    }
}

impl TryFrom<String> for FancyBarTabPlacement {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.eq_ignore_ascii_case("left") {
            Ok(Self::Left)
        } else if value.eq_ignore_ascii_case("center") {
            Ok(Self::Center)
        } else if value.eq_ignore_ascii_case("right") {
            Ok(Self::Right)
        } else {
            Err(format!(
                "invalid tab_placement '{}', expected 'left', 'center', or 'right'",
                value
            ))
        }
    }
}

#[derive(Debug, Clone, FromDynamic, ToDynamic)]
pub struct TabBarStyle {
    #[dynamic(default = "default_new_tab")]
    pub new_tab: String,
    #[dynamic(default = "default_new_tab")]
    pub new_tab_hover: String,
    #[dynamic(default = "default_window_hide")]
    pub window_hide: String,
    #[dynamic(default = "default_window_hide")]
    pub window_hide_hover: String,
    #[dynamic(default = "default_window_maximize")]
    pub window_maximize: String,
    #[dynamic(default = "default_window_maximize")]
    pub window_maximize_hover: String,
    #[dynamic(default = "default_window_close")]
    pub window_close: String,
    #[dynamic(default = "default_window_close")]
    pub window_close_hover: String,
}

impl Default for TabBarStyle {
    fn default() -> Self {
        Self {
            new_tab: default_new_tab(),
            new_tab_hover: default_new_tab(),
            window_hide: default_window_hide(),
            window_hide_hover: default_window_hide(),
            window_maximize: default_window_maximize(),
            window_maximize_hover: default_window_maximize(),
            window_close: default_window_close(),
            window_close_hover: default_window_close(),
        }
    }
}

fn default_new_tab() -> String {
    " + ".to_string()
}

fn default_window_hide() -> String {
    " . ".to_string()
}

fn default_window_maximize() -> String {
    " - ".to_string()
}

fn default_window_close() -> String {
    " X ".to_string()
}
