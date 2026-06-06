use crate::quad::TripleLayerQuadAllocator;
use crate::termwindow::render::fancy_tab_bar::scale_pixel_dim;
use crate::termwindow::render::RenderScreenLineParams;
use crate::utilsprites::RenderMetrics;
use config::ConfigHandle;
use mux::renderable::RenderableDimensions;
use wezterm_term::color::ColorAttribute;
use window::color::LinearRgba;

impl crate::TermWindow {
    pub fn paint_tab_bar(&mut self, layers: &mut TripleLayerQuadAllocator) -> anyhow::Result<()> {
        if self.config.use_fancy_tab_bar {
            if self.fancy_tab_bar.is_none() {
                let palette = self.palette().clone();
                let tab_bar = self.build_fancy_tab_bar(&palette)?;
                self.fancy_tab_bar.replace(tab_bar);
            }

            self.ui_items.append(&mut self.paint_fancy_tab_bar()?);
            return Ok(());
        }

        let border = self.get_os_border();

        let palette = self.palette().clone();
        let tab_bar_height = self.tab_bar_pixel_height()?;
        let tab_bar_y = if self.config.tab_bar_at_bottom {
            ((self.dimensions.pixel_height as f32) - (tab_bar_height + border.bottom.get() as f32))
                .max(0.)
        } else {
            border.top.get() as f32
        };

        // Register the tab bar location
        self.ui_items.append(&mut self.tab_bar.compute_ui_items(
            tab_bar_y as usize,
            self.render_metrics.cell_size.height as usize,
            self.render_metrics.cell_size.width as usize,
        ));

        let window_is_transparent =
            !self.window_background.is_empty() || self.config.window_background_opacity != 1.0;
        let gl_state = self.render_state.as_ref().unwrap();
        let white_space = gl_state.util_sprites.white_space.texture_coords();
        let filled_box = gl_state.util_sprites.filled_box.texture_coords();
        let default_bg = palette
            .resolve_bg(ColorAttribute::Default)
            .to_linear()
            .mul_alpha(if window_is_transparent {
                0.
            } else {
                self.config.text_background_opacity
            });

        self.render_screen_line(
            RenderScreenLineParams {
                top_pixel_y: tab_bar_y,
                left_pixel_x: 0.,
                pixel_width: self.dimensions.pixel_width as f32,
                stable_line_idx: None,
                line: self.tab_bar.line(),
                selection: 0..0,
                cursor: &Default::default(),
                palette: &palette,
                dims: &RenderableDimensions {
                    cols: self.dimensions.pixel_width
                        / self.render_metrics.cell_size.width as usize,
                    physical_top: 0,
                    scrollback_rows: 0,
                    scrollback_top: 0,
                    viewport_rows: 1,
                    dpi: self.terminal_size.dpi,
                    pixel_height: self.render_metrics.cell_size.height as usize,
                    pixel_width: self.terminal_size.pixel_width,
                    reverse_video: false,
                },
                config: &self.config,
                cursor_border_color: LinearRgba::default(),
                foreground: palette.foreground.to_linear(),
                pane: None,
                is_active: true,
                selection_fg: LinearRgba::default(),
                selection_bg: LinearRgba::default(),
                cursor_fg: LinearRgba::default(),
                cursor_bg: LinearRgba::default(),
                cursor_is_default_color: true,
                sync_panes_active: false,
                white_space,
                filled_box,
                window_is_transparent,
                default_bg,
                style: None,
                font: None,
                use_pixel_positioning: self.config.experimental_pixel_positioning,
                render_metrics: self.render_metrics,
                shape_key: None,
                password_input: false,
            },
            layers,
        )?;

        Ok(())
    }

    pub fn tab_bar_pixel_height_impl(
        config: &ConfigHandle,
        fontconfig: &wezterm_font::FontConfiguration,
        render_metrics: &RenderMetrics,
        dpi: usize,
    ) -> anyhow::Result<f32> {
        if config.use_fancy_tab_bar {
            let font = fontconfig.title_font()?;
            let cell_height = font.metrics().cell_height.get() as f32;
            let scale = dpi as f32 / ::window::DEFAULT_DPI as f32;

            if let Some(fancy) = config.fancy_bar.as_ref() {
                let ctx = config::DimensionContext {
                    dpi: dpi as f32,
                    pixel_max: cell_height * 2.0,
                    pixel_cell: cell_height,
                };

                // Container padding
                let pad = fancy.padding.as_ref();
                let ct = scale_pixel_dim(
                    pad.and_then(|p| p.top)
                        .unwrap_or(config::Dimension::Pixels(0.)),
                    scale,
                )
                .evaluate_as_pixels(ctx);
                let cb = scale_pixel_dim(
                    pad.and_then(|p| p.bottom)
                        .unwrap_or(config::Dimension::Pixels(0.)),
                    scale,
                )
                .evaluate_as_pixels(ctx);

                // Active tab total height (margin defaults to 0 when fancy_bar is set)
                let at = fancy.active_tab.as_ref();
                let active_h = cell_height
                    + Self::resolve_tab_dim(at, |t| t.margin.as_ref().and_then(|m| m.top), config::Dimension::Cells(0.), ctx, scale)
                    + Self::resolve_tab_dim(at, |t| t.margin.as_ref().and_then(|m| m.bottom), config::Dimension::Cells(0.), ctx, scale)
                    + Self::resolve_tab_dim(at, |t| t.padding.as_ref().and_then(|p| p.top), config::Dimension::Cells(0.2), ctx, scale)
                    + Self::resolve_tab_dim(at, |t| t.padding.as_ref().and_then(|p| p.bottom), config::Dimension::Cells(0.25), ctx, scale)
                    + Self::resolve_tab_dim(at, |t| t.border.as_ref().and_then(|b| b.top), config::Dimension::Pixels(1.), ctx, scale)
                    + Self::resolve_tab_dim(at, |t| t.border.as_ref().and_then(|b| b.bottom), config::Dimension::Pixels(1.), ctx, scale);

                // Inactive tab total height
                let it = fancy.inactive_tab.as_ref();
                let inactive_h = cell_height
                    + Self::resolve_tab_dim(it, |t| t.margin.as_ref().and_then(|m| m.top), config::Dimension::Cells(0.), ctx, scale)
                    + Self::resolve_tab_dim(it, |t| t.margin.as_ref().and_then(|m| m.bottom), config::Dimension::Cells(0.), ctx, scale)
                    + Self::resolve_tab_dim(it, |t| t.padding.as_ref().and_then(|p| p.top), config::Dimension::Cells(0.2), ctx, scale)
                    + Self::resolve_tab_dim(it, |t| t.padding.as_ref().and_then(|p| p.bottom), config::Dimension::Cells(0.25), ctx, scale)
                    + Self::resolve_tab_dim(it, |t| t.border.as_ref().and_then(|b| b.top), config::Dimension::Pixels(1.), ctx, scale)
                    + Self::resolve_tab_dim(it, |t| t.border.as_ref().and_then(|b| b.bottom), config::Dimension::Pixels(1.), ctx, scale);

                Ok((ct + cb + active_h.max(inactive_h)).ceil())
            } else {
                Ok((cell_height * 1.75).ceil())
            }
        } else {
            Ok(render_metrics.cell_size.height as f32)
        }
    }

    fn resolve_tab_dim(
        tab: Option<&config::FancyBarTabStyle>,
        getter: impl Fn(&config::FancyBarTabStyle) -> Option<config::Dimension>,
        default: config::Dimension,
        ctx: config::DimensionContext,
        scale: f32,
    ) -> f32 {
        scale_pixel_dim(tab.and_then(&getter).unwrap_or(default), scale)
            .evaluate_as_pixels(ctx)
    }

    pub fn tab_bar_pixel_height(&self) -> anyhow::Result<f32> {
        Self::tab_bar_pixel_height_impl(
            &self.config,
            &self.fonts,
            &self.render_metrics,
            self.dimensions.dpi,
        )
    }
}
