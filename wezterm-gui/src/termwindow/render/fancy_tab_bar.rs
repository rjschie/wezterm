use crate::customglyph::*;
use crate::tabbar::{TabBarItem, TabEntry};
use crate::termwindow::box_model::*;
use crate::termwindow::render::corners::*;

use crate::termwindow::render::window_buttons::window_button_element;
use crate::termwindow::{UIItem, UIItemType};
use crate::utilsprites::RenderMetrics;
use config::{
    Dimension, DimensionContext, FancyBarBoxColor, FancyBarBoxDimension, FancyBarButtonStyle,
    FancyBarConfig, FancyBarCornerRounding, FancyBarTabPlacement, FancyBarTabStyle,
    FancyBarTextAlign, TabBarColors,
};
use std::rc::Rc;
use wezterm_font::LoadedFont;
use wezterm_term::color::{ColorAttribute, ColorPalette};
use window::color::LinearRgba;
use window::{IntegratedTitleButtonAlignment, IntegratedTitleButtonStyle};

const MACOS_NATIVE_BUTTON_RESERVE_PX: f32 = 70.0;
const DEFAULT_H_PADDING: Dimension = Dimension::Cells(0.5);
const DEFAULT_V_PADDING_TOP: Dimension = Dimension::Cells(0.2);
const DEFAULT_V_PADDING_BOTTOM: Dimension = Dimension::Cells(0.25);

const X_BUTTON: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::One, BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Zero, BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Zero, BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

const PLUS_BUTTON: &[Poly] = &[
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Frac(1, 2), BlockCoord::Zero),
            PolyCommand::LineTo(BlockCoord::Frac(1, 2), BlockCoord::One),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
    Poly {
        path: &[
            PolyCommand::MoveTo(BlockCoord::Zero, BlockCoord::Frac(1, 2)),
            PolyCommand::LineTo(BlockCoord::One, BlockCoord::Frac(1, 2)),
        ],
        intensity: BlockAlpha::Full,
        style: PolyStyle::Outline,
    },
];

fn resolve_box_dim(fancy: Option<&FancyBarBoxDimension>, defaults: BoxDimension) -> BoxDimension {
    match fancy {
        Some(f) => BoxDimension {
            top: f.top.unwrap_or(defaults.top),
            bottom: f.bottom.unwrap_or(defaults.bottom),
            left: f.left.unwrap_or(defaults.left),
            right: f.right.unwrap_or(defaults.right),
        },
        None => defaults,
    }
}

fn resolve_corners(fancy: Option<&FancyBarCornerRounding>, defaults: Corners) -> Corners {
    match fancy {
        Some(f) => Corners {
            top_left: dim_to_corner(f.top_left, defaults.top_left, TOP_LEFT_ROUNDED_CORNER),
            top_right: dim_to_corner(f.top_right, defaults.top_right, TOP_RIGHT_ROUNDED_CORNER),
            bottom_left: dim_to_corner(
                f.bottom_left,
                defaults.bottom_left,
                BOTTOM_LEFT_ROUNDED_CORNER,
            ),
            bottom_right: dim_to_corner(
                f.bottom_right,
                defaults.bottom_right,
                BOTTOM_RIGHT_ROUNDED_CORNER,
            ),
        },
        None => defaults,
    }
}

fn dim_to_corner(
    dim: Option<Dimension>,
    default: SizedPoly,
    corner_poly: &'static [Poly],
) -> SizedPoly {
    match dim {
        Some(d) if d.is_zero() => SizedPoly::none(),
        Some(d) => SizedPoly {
            width: d,
            height: d,
            poly: corner_poly,
        },
        None => default,
    }
}

fn resolve_border_color(
    fancy: Option<&FancyBarBoxColor>,
    default: BorderColor,
) -> BorderColor {
    match fancy {
        Some(c) => BorderColor {
            top: resolve_color(c.top.as_ref(), || default.top),
            bottom: resolve_color(c.bottom.as_ref(), || default.bottom),
            left: resolve_color(c.left.as_ref(), || default.left),
            right: resolve_color(c.right.as_ref(), || default.right),
        },
        None => default,
    }
}

fn resolve_color(
    fancy_color: Option<&config::RgbaColor>,
    fallback: impl Fn() -> LinearRgba,
) -> LinearRgba {
    fancy_color
        .map(|c| c.to_linear())
        .unwrap_or_else(fallback)
}

pub(super) fn scale_pixel_dim(d: Dimension, scale: f32) -> Dimension {
    match d {
        Dimension::Pixels(n) => Dimension::Pixels(n * scale),
        other => other,
    }
}

fn scale_box_dim(bd: BoxDimension, scale: f32) -> BoxDimension {
    BoxDimension {
        left: scale_pixel_dim(bd.left, scale),
        top: scale_pixel_dim(bd.top, scale),
        right: scale_pixel_dim(bd.right, scale),
        bottom: scale_pixel_dim(bd.bottom, scale),
    }
}

fn scale_sized_poly(sp: SizedPoly, scale: f32) -> SizedPoly {
    SizedPoly {
        width: scale_pixel_dim(sp.width, scale),
        height: scale_pixel_dim(sp.height, scale),
        poly: sp.poly,
    }
}

fn scale_corners(c: Corners, scale: f32) -> Corners {
    Corners {
        top_left: scale_sized_poly(c.top_left, scale),
        top_right: scale_sized_poly(c.top_right, scale),
        bottom_left: scale_sized_poly(c.bottom_left, scale),
        bottom_right: scale_sized_poly(c.bottom_right, scale),
    }
}

fn default_tab_margin(v_margin: Dimension) -> BoxDimension {
    BoxDimension {
        left: Dimension::Cells(0.),
        right: Dimension::Cells(0.),
        top: v_margin,
        bottom: Dimension::Cells(0.),
    }
}

fn default_tab_padding() -> BoxDimension {
    BoxDimension {
        left: DEFAULT_H_PADDING,
        right: DEFAULT_H_PADDING,
        top: DEFAULT_V_PADDING_TOP,
        bottom: DEFAULT_V_PADDING_BOTTOM,
    }
}

fn default_tab_corners(active: bool) -> Corners {
    Corners {
        top_left: SizedPoly {
            width: DEFAULT_H_PADDING,
            height: DEFAULT_H_PADDING,
            poly: TOP_LEFT_ROUNDED_CORNER,
        },
        top_right: SizedPoly {
            width: DEFAULT_H_PADDING,
            height: DEFAULT_H_PADDING,
            poly: TOP_RIGHT_ROUNDED_CORNER,
        },
        bottom_left: if active {
            SizedPoly::none()
        } else {
            SizedPoly {
                width: Dimension::Cells(0.),
                height: Dimension::Cells(0.33),
                poly: &[],
            }
        },
        bottom_right: if active {
            SizedPoly::none()
        } else {
            SizedPoly {
                width: Dimension::Cells(0.),
                height: Dimension::Cells(0.33),
                poly: &[],
            }
        },
    }
}

struct ResolvedBoxStyle {
    margin: BoxDimension,
    padding: BoxDimension,
    corners: Option<Corners>,
    border: BoxDimension,
    border_color: BorderColor,
}

fn resolve_button_box_style(
    btn: Option<&FancyBarButtonStyle>,
    default_margin: BoxDimension,
    default_padding: BoxDimension,
    scale: f32,
) -> ResolvedBoxStyle {
    ResolvedBoxStyle {
        margin: scale_box_dim(
            resolve_box_dim(btn.and_then(|b| b.margin.as_ref()), default_margin),
            scale,
        ),
        padding: scale_box_dim(
            resolve_box_dim(btn.and_then(|b| b.padding.as_ref()), default_padding),
            scale,
        ),
        corners: btn
            .and_then(|b| b.rounding.as_ref())
            .map(|r| scale_corners(resolve_corners(Some(r), Corners::default()), scale)),
        border: scale_box_dim(
            resolve_box_dim(
                btn.and_then(|b| b.border.as_ref()),
                BoxDimension::default(),
            ),
            scale,
        ),
        border_color: resolve_border_color(
            btn.and_then(|b| b.border_color.as_ref()),
            BorderColor::default(),
        ),
    }
}

fn build_status_element(
    element: Element,
    fancy: Option<&FancyBarConfig>,
    bar_colors: &ElementColors,
) -> Element {
    element
        .item_type(UIItemType::TabBar(TabBarItem::None))
        .line_height(if fancy.is_some() { None } else { Some(1.75) })
        .margin(BoxDimension {
            left: Dimension::Cells(0.),
            right: Dimension::Cells(0.),
            top: Dimension::Cells(0.0),
            bottom: Dimension::Cells(0.),
        })
        .padding(BoxDimension {
            left: DEFAULT_H_PADDING,
            right: Dimension::Cells(0.),
            top: Dimension::Cells(0.),
            bottom: Dimension::Cells(0.),
        })
        .border(BoxDimension::new(Dimension::Pixels(0.)))
        .colors(bar_colors.clone())
}

fn build_new_tab_button(
    item: &TabEntry,
    font: &Rc<LoadedFont>,
    metrics: &RenderMetrics,
    fancy: Option<&FancyBarConfig>,
    colors: &TabBarColors,
    scale: f32,
    default_v_margin: Dimension,
    fonts: &Rc<wezterm_font::FontConfiguration>,
) -> Element {
    let fancy_btn = fancy.and_then(|f| f.new_tab_button.as_ref());

    let btn_font = match fancy_btn.and_then(|b| b.font_size) {
        Some(size) => fonts
            .title_font_with_size(size)
            .unwrap_or_else(|_| Rc::clone(font)),
        None => Rc::clone(font),
    };
    let btn_metrics = match fancy_btn.and_then(|b| b.font_size) {
        Some(_) => RenderMetrics::with_font_metrics(&btn_font.metrics()),
        None => metrics.clone(),
    };

    let content = match fancy_btn.and_then(|b| b.text.as_ref()) {
        Some(text) => ElementContent::Text(text.clone()),
        None => ElementContent::Poly {
            line_width: btn_metrics.underline_height.max(2),
            poly: SizedPoly {
                poly: PLUS_BUTTON,
                width: Dimension::Pixels(btn_metrics.cell_size.height as f32 / 2.),
                height: Dimension::Pixels(btn_metrics.cell_size.height as f32 / 2.),
            },
        },
    };

    let new_tab = colors.new_tab();
    let new_tab_hover = colors.new_tab_hover();

    let btn_bg = resolve_color(
        fancy_btn.and_then(|b| b.bg_color.as_ref()),
        || new_tab.bg_color.to_linear(),
    );
    let btn_fg = resolve_color(
        fancy_btn.and_then(|b| b.fg_color.as_ref()),
        || new_tab.fg_color.to_linear(),
    );
    let hover_bg = resolve_color(
        fancy_btn.and_then(|b| b.hover_bg_color.as_ref()),
        || new_tab_hover.bg_color.to_linear(),
    );
    let hover_fg = resolve_color(
        fancy_btn.and_then(|b| b.hover_fg_color.as_ref()),
        || new_tab_hover.fg_color.to_linear(),
    );

    let style = resolve_button_box_style(
        fancy_btn,
        BoxDimension {
            left: DEFAULT_H_PADDING,
            right: Dimension::Cells(0.),
            top: default_v_margin,
            bottom: Dimension::Cells(0.),
        },
        BoxDimension {
            left: DEFAULT_H_PADDING,
            right: DEFAULT_H_PADDING,
            top: DEFAULT_V_PADDING_TOP,
            bottom: DEFAULT_V_PADDING_BOTTOM,
        },
        scale,
    );

    Element::new(&btn_font, content)
        .vertical_align(VerticalAlign::Middle)
        .item_type(UIItemType::TabBar(item.item.clone()))
        .margin(style.margin)
        .padding(style.padding)
        .border(style.border)
        .border_corners(style.corners)
        .colors(ElementColors {
            border: style.border_color,
            bg: btn_bg.into(),
            text: btn_fg.into(),
        })
        .hover_colors(Some(ElementColors {
            border: style.border_color,
            bg: hover_bg.into(),
            text: hover_fg.into(),
        }))
}

fn build_active_tab(
    item: &TabEntry,
    element: Element,
    fancy_tab: Option<&FancyBarTabStyle>,
    active_tab_colors: &config::TabBarColor,
    bg_color: Option<termwiz::color::SrgbaTuple>,
    fg_color: Option<termwiz::color::SrgbaTuple>,
    default_v_margin: Dimension,
    scale: f32,
) -> Element {
    let margin = scale_box_dim(
        resolve_box_dim(
            fancy_tab.and_then(|t| t.margin.as_ref()),
            default_tab_margin(default_v_margin),
        ),
        scale,
    );
    let padding = scale_box_dim(
        resolve_box_dim(
            fancy_tab.and_then(|t| t.padding.as_ref()),
            default_tab_padding(),
        ),
        scale,
    );
    let corners = scale_corners(
        resolve_corners(
            fancy_tab.and_then(|t| t.rounding.as_ref()),
            default_tab_corners(true),
        ),
        scale,
    );
    let border = scale_box_dim(
        resolve_box_dim(
            fancy_tab.and_then(|t| t.border.as_ref()),
            BoxDimension::new(Dimension::Pixels(1.)),
        ),
        scale,
    );

    let tab_bg = bg_color.unwrap_or_else(|| {
        fancy_tab
            .and_then(|t| t.bg_color.as_ref())
            .cloned()
            .unwrap_or(active_tab_colors.bg_color)
            .into()
    });
    let tab_fg = fg_color.unwrap_or_else(|| {
        fancy_tab
            .and_then(|t| t.fg_color.as_ref())
            .cloned()
            .unwrap_or(active_tab_colors.fg_color)
            .into()
    });
    let border_color = resolve_border_color(
        fancy_tab.and_then(|t| t.border_color.as_ref()),
        BorderColor::new(tab_bg.to_linear()),
    );

    let mut elem = element
        .vertical_align(VerticalAlign::Bottom)
        .item_type(UIItemType::TabBar(item.item.clone()))
        .margin(margin)
        .padding(padding)
        .border(border)
        .border_corners(Some(corners))
        .colors(ElementColors {
            border: border_color,
            bg: tab_bg.to_linear().into(),
            text: tab_fg.to_linear().into(),
        });

    if let Some(ft) = fancy_tab {
        if ft.hover_bg_color.is_some() || ft.hover_fg_color.is_some() {
            let hover_bg = resolve_color(ft.hover_bg_color.as_ref(), || tab_bg.to_linear());
            let hover_fg = resolve_color(ft.hover_fg_color.as_ref(), || tab_fg.to_linear());
            elem = elem.hover_colors(Some(ElementColors {
                border: BorderColor::new(hover_bg),
                bg: hover_bg.into(),
                text: hover_fg.into(),
            }));
        }
    }

    elem
}

fn build_inactive_tab(
    item: &TabEntry,
    element: Element,
    fancy_tab: Option<&FancyBarTabStyle>,
    colors: &TabBarColors,
    bg_color: Option<termwiz::color::SrgbaTuple>,
    fg_color: Option<termwiz::color::SrgbaTuple>,
    default_v_margin: Dimension,
    scale: f32,
) -> Element {
    let inactive_tab = colors.inactive_tab();

    let margin = scale_box_dim(
        resolve_box_dim(
            fancy_tab.and_then(|t| t.margin.as_ref()),
            default_tab_margin(default_v_margin),
        ),
        scale,
    );
    let padding = scale_box_dim(
        resolve_box_dim(
            fancy_tab.and_then(|t| t.padding.as_ref()),
            default_tab_padding(),
        ),
        scale,
    );
    let corners = scale_corners(
        resolve_corners(
            fancy_tab.and_then(|t| t.rounding.as_ref()),
            default_tab_corners(false),
        ),
        scale,
    );
    let border = scale_box_dim(
        resolve_box_dim(
            fancy_tab.and_then(|t| t.border.as_ref()),
            BoxDimension::new(Dimension::Pixels(1.)),
        ),
        scale,
    );

    let tab_bg = bg_color.unwrap_or_else(|| {
        fancy_tab
            .and_then(|t| t.bg_color.as_ref())
            .cloned()
            .unwrap_or(inactive_tab.bg_color)
            .into()
    });
    let tab_fg = fg_color.unwrap_or_else(|| {
        fancy_tab
            .and_then(|t| t.fg_color.as_ref())
            .cloned()
            .unwrap_or(inactive_tab.fg_color)
            .into()
    });
    let bg_linear = tab_bg.to_linear();
    let border_color = resolve_border_color(
        fancy_tab.and_then(|t| t.border_color.as_ref()),
        BorderColor {
            left: bg_linear,
            right: colors.inactive_tab_edge().to_linear(),
            top: bg_linear,
            bottom: bg_linear,
        },
    );

    let inactive_tab_hover = colors.inactive_tab_hover();
    let hover_bg = resolve_color(
        fancy_tab.and_then(|t| t.hover_bg_color.as_ref()),
        || {
            bg_color
                .unwrap_or_else(|| inactive_tab_hover.bg_color.into())
                .to_linear()
        },
    );
    let hover_fg = resolve_color(
        fancy_tab.and_then(|t| t.hover_fg_color.as_ref()),
        || {
            fg_color
                .unwrap_or_else(|| inactive_tab_hover.fg_color.into())
                .to_linear()
        },
    );

    element
        .vertical_align(VerticalAlign::Bottom)
        .item_type(UIItemType::TabBar(item.item.clone()))
        .margin(margin)
        .padding(padding)
        .border(border)
        .border_corners(Some(corners))
        .colors(ElementColors {
            border: border_color,
            bg: bg_linear.into(),
            text: tab_fg.to_linear().into(),
        })
        .hover_colors(Some(ElementColors {
            border: BorderColor::new(hover_bg),
            bg: hover_bg.into(),
            text: hover_fg.into(),
        }))
}

impl crate::TermWindow {
    pub fn invalidate_fancy_tab_bar(&mut self) {
        self.fancy_tab_bar.take();
    }

    pub fn build_fancy_tab_bar(&self, palette: &ColorPalette) -> anyhow::Result<ComputedElement> {
        let tab_bar_height = self.tab_bar_pixel_height()?;
        let font = self.fonts.title_font()?;
        let metrics = RenderMetrics::with_font_metrics(&font.metrics());
        let scale = self.dimensions.dpi as f32 / ::window::DEFAULT_DPI as f32;
        let items = self.tab_bar.items();
        let colors = self
            .config
            .colors
            .as_ref()
            .and_then(|c| c.tab_bar.as_ref())
            .cloned()
            .unwrap_or_else(TabBarColors::default);

        let fancy = self.config.fancy_bar.as_ref();
        let use_full_width = fancy.and_then(|f| f.use_full_width).unwrap_or(false);

        let mut left_status = vec![];
        let mut left_eles = vec![];
        let mut right_eles = vec![];

        // Background color: fancy_bar -> window_frame
        let bar_colors = ElementColors {
            border: BorderColor::default(),
            bg: {
                let focused = self.focused.is_some();
                if let Some(c) = fancy.and_then(|f| {
                    if focused {
                        f.background.as_ref()
                    } else {
                        f.inactive_background.as_ref().or(f.background.as_ref())
                    }
                }) {
                    c.to_linear().into()
                } else if focused {
                    self.config.window_frame.active_titlebar_bg.to_linear().into()
                } else {
                    self.config
                        .window_frame
                        .inactive_titlebar_bg
                        .to_linear()
                        .into()
                }
            },
            text: if self.focused.is_some() {
                self.config.window_frame.active_titlebar_fg
            } else {
                self.config.window_frame.inactive_titlebar_fg
            }
            .to_linear()
            .into(),
        };

        // When fancy_bar is configured, default vertical margins are 0;
        // spacing is controlled via fancy_bar.padding instead.
        let default_v_margin = if fancy.is_some() {
            Dimension::Cells(0.)
        } else {
            Dimension::Cells(0.2)
        };

        let active_tab_colors = colors.active_tab();

        let item_to_elem = |item: &TabEntry| -> Element {
            let element = Element::with_line(&font, &item.title, palette);

            let bg_color = item
                .title
                .get_cell(0)
                .and_then(|c| match c.attrs().background() {
                    ColorAttribute::Default => None,
                    col => Some(palette.resolve_bg(col)),
                });
            let fg_color = item
                .title
                .get_cell(0)
                .and_then(|c| match c.attrs().foreground() {
                    ColorAttribute::Default => None,
                    col => Some(palette.resolve_fg(col)),
                });

            match item.item {
                TabBarItem::RightStatus | TabBarItem::LeftStatus | TabBarItem::None => {
                    build_status_element(element, fancy, &bar_colors)
                }

                TabBarItem::NewTabButton => build_new_tab_button(
                    item,
                    &font,
                    &metrics,
                    fancy,
                    &colors,
                    scale,
                    default_v_margin,
                    &self.fonts,
                ),

                TabBarItem::Tab { active, .. } if active => build_active_tab(
                    item,
                    element,
                    fancy.and_then(|f| f.active_tab.as_ref()),
                    &active_tab_colors,
                    bg_color,
                    fg_color,
                    default_v_margin,
                    scale,
                ),

                TabBarItem::Tab { .. } => build_inactive_tab(
                    item,
                    element,
                    fancy.and_then(|f| f.inactive_tab.as_ref()),
                    &colors,
                    bg_color,
                    fg_color,
                    default_v_margin,
                    scale,
                ),

                TabBarItem::WindowButton(button) => window_button_element(
                    button,
                    self.window_state.contains(window::WindowState::MAXIMIZED),
                    &font,
                    &metrics,
                    &self.config,
                ),
            }
        };

        let num_tabs: f32 = items
            .iter()
            .map(|item| match item.item {
                TabBarItem::Tab { .. } => 1.,
                _ => 0.,
            })
            .sum();

        // Resolve tab min/max width from fancy_bar config
        let tab_width_ctx = DimensionContext {
            dpi: self.dimensions.dpi as f32,
            pixel_max: self.dimensions.pixel_width as f32,
            pixel_cell: metrics.cell_size.width as f32,
        };
        let cfg_tab_min_width = fancy
            .and_then(|f| f.tab_min_width)
            .map(|d| scale_pixel_dim(d, scale).evaluate_as_pixels(tab_width_ctx));
        let cfg_tab_max_width = fancy
            .and_then(|f| f.tab_max_width)
            .map(|d| scale_pixel_dim(d, scale).evaluate_as_pixels(tab_width_ctx))
            .unwrap_or(self.config.tab_max_width as f32 * metrics.cell_size.width as f32);

        let max_tab_width = if use_full_width {
            // Will be computed after layout elements are gathered
            f32::MAX
        } else {
            let num_tab_like: f32 = items
                .iter()
                .map(|item| match item.item {
                    TabBarItem::NewTabButton | TabBarItem::Tab { .. } => 1.,
                    _ => 0.,
                })
                .sum();
            let computed = ((self.dimensions.pixel_width as f32 / num_tab_like)
                - (1.5 * metrics.cell_size.width as f32))
                .max(0.);
            computed.min(cfg_tab_max_width)
        };

        // Reserve space for the native titlebar buttons
        if self
            .config
            .window_decorations
            .contains(::window::WindowDecorations::INTEGRATED_BUTTONS)
            && self.config.integrated_title_button_style == IntegratedTitleButtonStyle::MacOsNative
            && !self.window_state.contains(window::WindowState::FULL_SCREEN)
        {
            left_status.push(
                Element::new(&font, ElementContent::Text("".to_string())).margin(BoxDimension {
                    left: Dimension::Cells(4.0),
                    right: Dimension::Cells(0.),
                    top: Dimension::Cells(0.),
                    bottom: Dimension::Cells(0.),
                }),
            );
        }

        // Compute separator height: tab bar height minus container top/bottom padding
        let container_padding = fancy.and_then(|f| f.padding.as_ref());
        let height_ctx = DimensionContext {
            dpi: self.dimensions.dpi as f32,
            pixel_max: self.dimensions.pixel_height as f32,
            pixel_cell: metrics.cell_size.height as f32,
        };
        let top_pad_px = scale_pixel_dim(
            container_padding.and_then(|p| p.top).unwrap_or(Dimension::Cells(0.)),
            scale,
        ).evaluate_as_pixels(height_ctx);
        let bottom_pad_px = scale_pixel_dim(
            container_padding.and_then(|p| p.bottom).unwrap_or(Dimension::Cells(0.)),
            scale,
        ).evaluate_as_pixels(height_ctx);
        let separator_height = (tab_bar_height - top_pad_px - bottom_pad_px).max(0.);

        // Track tab indices for separator insertion
        let mut tab_count = 0u32;
        let mut tab_group_eles: Vec<Element> = vec![];

        for item in items {
            match item.item {
                TabBarItem::LeftStatus => left_status.push(item_to_elem(item)),
                TabBarItem::None | TabBarItem::RightStatus => right_eles.push(item_to_elem(item)),
                TabBarItem::WindowButton(_) => {
                    if self.config.integrated_title_button_alignment
                        == IntegratedTitleButtonAlignment::Left
                    {
                        left_eles.push(item_to_elem(item))
                    } else {
                        right_eles.push(item_to_elem(item))
                    }
                }
                TabBarItem::Tab { tab_idx, active } => {
                    // Insert tab separator before non-first tabs
                    if tab_count > 0 {
                        if let Some(sep) = fancy.and_then(|f| f.tab_separator.as_ref()) {
                            let thickness = scale_pixel_dim(
                                sep.thickness.unwrap_or(Dimension::Pixels(0.)),
                                scale,
                            );
                            let sep_color = sep
                                .color
                                .as_ref()
                                .map(|c| c.to_linear().into())
                                .unwrap_or(InheritableColor::Inherited);
                            let sep_margin = scale_box_dim(resolve_box_dim(
                                sep.margin.as_ref(),
                                BoxDimension::default(),
                            ), scale);
                            tab_group_eles.push(
                                Element::new(&font, ElementContent::Children(vec![]))
                                    .min_width(Some(thickness))
                                    .min_height(Some(Dimension::Pixels(separator_height)))
                                    .colors(ElementColors {
                                        border: BorderColor::default(),
                                        bg: sep_color,
                                        text: InheritableColor::Inherited,
                                    })
                                    .margin(sep_margin),
                            );
                        }
                    }
                    tab_count += 1;

                    let mut elem = item_to_elem(item);
                    elem.max_width = Some(Dimension::Pixels(max_tab_width));
                    if let Some(min_w) = cfg_tab_min_width {
                        elem.min_width = Some(Dimension::Pixels(min_w));
                    }
                    let tab_h_align = match fancy
                        .map(|f| f.tab_text_align)
                        .unwrap_or_default()
                    {
                        FancyBarTextAlign::Left => HorizontalAlign::Left,
                        FancyBarTextAlign::Center => HorizontalAlign::Center,
                        FancyBarTextAlign::Right => HorizontalAlign::Right,
                    };
                    elem.content = match elem.content {
                        ElementContent::Text(_) => unreachable!(),
                        ElementContent::Poly { .. } => unreachable!(),
                        ElementContent::Children(mut kids) => {
                            for kid in kids.iter_mut() {
                                kid.horizontal_align = tab_h_align;
                            }
                            if self.config.show_close_tab_button_in_tabs {
                                kids.push(make_x_button(
                                    &font, &metrics, &colors, tab_idx, active, fancy, scale,
                                    &self.fonts,
                                ));
                            }
                            ElementContent::Children(kids)
                        }
                    };

                    tab_group_eles.push(elem);
                }
                _ => tab_group_eles.push(item_to_elem(item)),
            }
        }

        let window_buttons_at_left = self
            .config
            .window_decorations
            .contains(window::WindowDecorations::INTEGRATED_BUTTONS)
            && (self.config.integrated_title_button_alignment
                == IntegratedTitleButtonAlignment::Left
                || self.config.integrated_title_button_style
                    == IntegratedTitleButtonStyle::MacOsNative);

        // Container padding: fancy_bar.padding -> defaults
        let left_padding = scale_pixel_dim(if window_buttons_at_left {
            if self.config.integrated_title_button_style == IntegratedTitleButtonStyle::MacOsNative
            {
                if !self.window_state.contains(window::WindowState::FULL_SCREEN) {
                    Dimension::Pixels(MACOS_NATIVE_BUTTON_RESERVE_PX)
                } else {
                    container_padding
                        .and_then(|p| p.left)
                        .unwrap_or(DEFAULT_H_PADDING)
                }
            } else {
                container_padding
                    .and_then(|p| p.left)
                    .unwrap_or(Dimension::Pixels(0.0))
            }
        } else {
            container_padding
                .and_then(|p| p.left)
                .unwrap_or(DEFAULT_H_PADDING)
        }, scale);
        let right_padding = scale_pixel_dim(container_padding
            .and_then(|p| p.right)
            .unwrap_or(Dimension::Cells(0.)), scale);
        let top_padding = scale_pixel_dim(container_padding
            .and_then(|p| p.top)
            .unwrap_or(Dimension::Cells(0.)), scale);
        let bottom_padding = scale_pixel_dim(container_padding
            .and_then(|p| p.bottom)
            .unwrap_or(Dimension::Cells(0.)), scale);

        // use_full_width: compute per-tab width and apply
        if use_full_width && num_tabs > 0. {
            let width_ctx = DimensionContext {
                dpi: self.dimensions.dpi as f32,
                pixel_max: self.dimensions.pixel_width as f32,
                pixel_cell: metrics.cell_size.width as f32,
            };

            let pad_left = left_padding.evaluate_as_pixels(width_ctx);
            let pad_right = right_padding.evaluate_as_pixels(width_ctx);

            // New tab button outer width: content + padding + border + margin
            let ntb_width = if self.config.show_new_tab_button_in_tab_bar {
                let fancy_btn = fancy.and_then(|f| f.new_tab_button.as_ref());
                let ntb_content = match fancy_btn.and_then(|b| b.text.as_ref()) {
                    Some(text) => text.len() as f32 * metrics.cell_size.width as f32,
                    None => metrics.cell_size.height as f32 / 2.,
                };
                let ntb_style = resolve_button_box_style(
                    fancy_btn,
                    BoxDimension {
                        left: DEFAULT_H_PADDING,
                        right: Dimension::Cells(0.),
                        top: Dimension::Cells(0.),
                        bottom: Dimension::Cells(0.),
                    },
                    BoxDimension {
                        left: DEFAULT_H_PADDING,
                        right: DEFAULT_H_PADDING,
                        top: Dimension::Cells(0.),
                        bottom: Dimension::Cells(0.),
                    },
                    scale,
                );
                let ntb_border = scale_box_dim(
                    BoxDimension::new(Dimension::Pixels(1.)),
                    scale,
                );
                ntb_content
                    + ntb_style.margin.left.evaluate_as_pixels(width_ctx)
                    + ntb_style.margin.right.evaluate_as_pixels(width_ctx)
                    + ntb_style.padding.left.evaluate_as_pixels(width_ctx)
                    + ntb_style.padding.right.evaluate_as_pixels(width_ctx)
                    + ntb_border.left.evaluate_as_pixels(width_ctx)
                    + ntb_border.right.evaluate_as_pixels(width_ctx)
            } else {
                0.
            };

            // Per-tab horizontal margin
            let tab_margin = |tab: Option<&config::FancyBarTabStyle>| -> f32 {
                let m = scale_box_dim(resolve_box_dim(
                    tab.and_then(|t| t.margin.as_ref()),
                    BoxDimension {
                        left: Dimension::Cells(0.),
                        right: Dimension::Cells(0.),
                        top: Dimension::Cells(0.),
                        bottom: Dimension::Cells(0.),
                    },
                ), scale);
                m.left.evaluate_as_pixels(width_ctx) + m.right.evaluate_as_pixels(width_ctx)
            };
            let active_tab_margin = tab_margin(fancy.and_then(|f| f.active_tab.as_ref()));
            let inactive_tab_margin = tab_margin(fancy.and_then(|f| f.inactive_tab.as_ref()));
            // 1 active tab, rest inactive
            let total_tab_margins = active_tab_margin + inactive_tab_margin * (num_tabs - 1.);

            // Separator overhead: (num_tabs - 1) separators
            let sep_overhead = if let Some(sep) = fancy.and_then(|f| f.tab_separator.as_ref()) {
                let thickness = scale_pixel_dim(
                    sep.thickness.unwrap_or(Dimension::Pixels(0.)),
                    scale,
                ).evaluate_as_pixels(width_ctx);
                let sep_m = scale_box_dim(resolve_box_dim(
                    sep.margin.as_ref(),
                    BoxDimension::default(),
                ), scale);
                let sep_total = thickness
                    + sep_m.left.evaluate_as_pixels(width_ctx)
                    + sep_m.right.evaluate_as_pixels(width_ctx);
                sep_total * (num_tabs - 1.)
            } else {
                0.
            };

            let available = (self.dimensions.pixel_width as f32
                - pad_left
                - pad_right
                - ntb_width
                - total_tab_margins
                - sep_overhead)
                .max(0.);
            let per_tab = available / num_tabs;

            for elem in tab_group_eles.iter_mut() {
                if matches!(
                    elem.item_type,
                    Some(UIItemType::TabBar(TabBarItem::Tab { .. }))
                ) {
                    elem.min_width = Some(Dimension::Pixels(per_tab));
                    elem.max_width = Some(Dimension::Pixels(per_tab));
                }
            }
        }

        let tab_placement_align = match fancy.map(|f| f.tab_placement).unwrap_or_default() {
            FancyBarTabPlacement::Left => HorizontalAlign::Left,
            FancyBarTabPlacement::Center => HorizontalAlign::Center,
            FancyBarTabPlacement::Right => HorizontalAlign::Right,
        };
        let tab_group = Element::new(&font, ElementContent::Children(tab_group_eles))
            .horizontal_align(tab_placement_align)
            .vertical_align(VerticalAlign::Bottom)
            .colors(bar_colors.clone());
        left_eles.push(tab_group);

        let mut children = vec![];

        if !left_status.is_empty() {
            children.push(
                Element::new(&font, ElementContent::Children(left_status))
                    .colors(bar_colors.clone()),
            );
        }

        children.push(
            Element::new(&font, ElementContent::Children(left_eles))
                .vertical_align(VerticalAlign::Bottom)
                .colors(bar_colors.clone())
                .min_width(Some(Dimension::Pixels(self.dimensions.pixel_width as f32)))
                .padding(BoxDimension {
                    left: left_padding,
                    right: right_padding,
                    top: top_padding,
                    bottom: bottom_padding,
                })
                .zindex(1),
        );
        children.push(
            Element::new(&font, ElementContent::Children(right_eles))
                .colors(bar_colors.clone())
                .float(Float::Right),
        );

        let content = ElementContent::Children(children);

        let tabs = Element::new(&font, content)
            .display(DisplayType::Block)
            .item_type(UIItemType::TabBar(TabBarItem::None))
            .min_width(Some(Dimension::Pixels(self.dimensions.pixel_width as f32)))
            .min_height(Some(Dimension::Pixels(tab_bar_height)))
            .vertical_align(VerticalAlign::Bottom)
            .colors(bar_colors);

        let border = self.get_os_border();

        let mut computed = self.compute_element(
            &LayoutContext {
                height: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_height as f32,
                    pixel_cell: metrics.cell_size.height as f32,
                },
                width: DimensionContext {
                    dpi: self.dimensions.dpi as f32,
                    pixel_max: self.dimensions.pixel_width as f32,
                    pixel_cell: metrics.cell_size.width as f32,
                },
                bounds: euclid::rect(
                    border.left.get() as f32,
                    0.,
                    self.dimensions.pixel_width as f32 - (border.left + border.right).get() as f32,
                    tab_bar_height,
                ),
                metrics: &metrics,
                gl_state: self.render_state.as_ref().unwrap(),
                zindex: 10,
            },
            &tabs,
        )?;

        computed.translate(euclid::vec2(
            0.,
            if self.config.tab_bar_at_bottom {
                self.dimensions.pixel_height as f32
                    - (computed.bounds.height() + border.bottom.get() as f32)
            } else {
                border.top.get() as f32
            },
        ));

        Ok(computed)
    }

    pub fn paint_fancy_tab_bar(&self) -> anyhow::Result<Vec<UIItem>> {
        let computed = self.fancy_tab_bar.as_ref().ok_or_else(|| {
            anyhow::anyhow!("paint_fancy_tab_bar called but fancy_tab_bar is None")
        })?;
        let ui_items = computed.ui_items();

        let gl_state = self.render_state.as_ref().unwrap();
        self.render_element(&computed, gl_state, None)?;

        Ok(ui_items)
    }
}

fn make_x_button(
    font: &Rc<LoadedFont>,
    metrics: &RenderMetrics,
    colors: &TabBarColors,
    tab_idx: usize,
    active: bool,
    fancy: Option<&FancyBarConfig>,
    scale: f32,
    fonts: &Rc<wezterm_font::FontConfiguration>,
) -> Element {
    let fancy_btn = if active {
        fancy.and_then(|f| f.active_close_tab_button.as_ref())
    } else {
        fancy.and_then(|f| f.inactive_close_tab_button.as_ref())
    };

    let btn_font = match fancy_btn.and_then(|b| b.font_size) {
        Some(size) => fonts.title_font_with_size(size).unwrap_or_else(|_| Rc::clone(font)),
        None => Rc::clone(font),
    };
    let btn_metrics = match fancy_btn.and_then(|b| b.font_size) {
        Some(_) => RenderMetrics::with_font_metrics(&btn_font.metrics()),
        None => metrics.clone(),
    };

    let content = match fancy_btn.and_then(|b| b.text.as_ref()) {
        Some(text) => ElementContent::Text(text.clone()),
        None => ElementContent::Poly {
            line_width: btn_metrics.underline_height.max(2),
            poly: SizedPoly {
                poly: X_BUTTON,
                width: Dimension::Pixels(btn_metrics.cell_size.height as f32 / 2.),
                height: Dimension::Pixels(btn_metrics.cell_size.height as f32 / 2.),
            },
        },
    };

    let inactive_tab_hover = colors.inactive_tab_hover();
    let active_tab = colors.active_tab();

    let hover_bg = resolve_color(
        fancy_btn.and_then(|b| b.hover_bg_color.as_ref()),
        || {
            (if active {
                inactive_tab_hover.bg_color
            } else {
                active_tab.bg_color
            })
            .to_linear()
        },
    );
    let hover_fg = resolve_color(
        fancy_btn.and_then(|b| b.hover_fg_color.as_ref()),
        || {
            (if active {
                inactive_tab_hover.fg_color
            } else {
                active_tab.fg_color
            })
            .to_linear()
        },
    );

    let style = resolve_button_box_style(
        fancy_btn,
        BoxDimension {
            left: DEFAULT_H_PADDING,
            right: Dimension::Cells(0.),
            top: Dimension::Cells(0.),
            bottom: Dimension::Cells(0.),
        },
        BoxDimension {
            left: Dimension::Cells(0.25),
            right: Dimension::Cells(0.25),
            top: Dimension::Cells(0.25),
            bottom: Dimension::Cells(0.25),
        },
        scale,
    );

    let mut elem = Element::new(&btn_font, content)
        .zindex(1)
        .vertical_align(VerticalAlign::Middle)
        .float(Float::Right)
        .item_type(UIItemType::CloseTab(tab_idx))
        .hover_colors(Some(ElementColors {
            border: style.border_color,
            bg: hover_bg.into(),
            text: hover_fg.into(),
        }))
        .padding(style.padding)
        .margin(style.margin)
        .border(style.border)
        .border_corners(style.corners);

    // Apply custom colors if set
    if let Some(btn) = fancy_btn {
        if btn.bg_color.is_some() || btn.fg_color.is_some() {
            let bg = btn
                .bg_color
                .as_ref()
                .map(|c| c.to_linear().into())
                .unwrap_or(InheritableColor::Inherited);
            let fg = btn
                .fg_color
                .as_ref()
                .map(|c| c.to_linear().into())
                .unwrap_or(InheritableColor::Inherited);
            elem = elem.colors(ElementColors {
                border: style.border_color,
                bg,
                text: fg,
            });
        }
    }

    elem
}
