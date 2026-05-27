pub mod groups;
pub mod types;

pub use groups::*;
pub use types::*;

use crate::render_tree::node::Display;

// ---------------------------------------------------------------------------
// Style builder -- the user-facing API.
//
// `Style` is a temporary object. When applied to a node via
// `RenderTree::apply_style`, its `Option<Group>` fields are scattered into
// the corresponding `SecondaryMap`s and the node's `StyleFlags` bitfield is
// updated.
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct Style {
    pub(crate) display: Display,
    pub(crate) size: Option<SizeStyle>,
    pub(crate) spacing: Option<SpacingStyle>,
    pub(crate) flex: Option<FlexStyle>,
    pub(crate) background: Option<BackgroundStyle>,
    pub(crate) border: Option<BorderStyle>,
    pub(crate) text_style: Option<TextStyleGroup>,
    pub(crate) position: Option<PositionStyle>,
    pub(crate) effect: Option<EffectStyle>,
    pub(crate) overflow: Option<OverflowStyle>,
}

impl Style {
    pub fn new() -> Self {
        Self::default()
    }

    // -- display (inline on RenderNode) ------------------------------------

    pub fn display(mut self, d: Display) -> Self {
        self.display = d;
        self
    }

    // -- size --------------------------------------------------------------

    pub fn width(mut self, w: Dimension) -> Self {
        self.size.get_or_insert_with(Default::default).width = w;
        self
    }
    pub fn height(mut self, h: Dimension) -> Self {
        self.size.get_or_insert_with(Default::default).height = h;
        self
    }
    pub fn min_width(mut self, w: Dimension) -> Self {
        self.size.get_or_insert_with(Default::default).min_width = w;
        self
    }
    pub fn max_width(mut self, w: Dimension) -> Self {
        self.size.get_or_insert_with(Default::default).max_width = w;
        self
    }
    pub fn min_height(mut self, h: Dimension) -> Self {
        self.size.get_or_insert_with(Default::default).min_height = h;
        self
    }
    pub fn max_height(mut self, h: Dimension) -> Self {
        self.size.get_or_insert_with(Default::default).max_height = h;
        self
    }

    // -- spacing -----------------------------------------------------------

    pub fn padding(mut self, t: f32, r: f32, b: f32, l: f32) -> Self {
        self.spacing.get_or_insert_with(Default::default).padding = [t, r, b, l];
        self
    }
    pub fn padding_all(self, p: f32) -> Self {
        self.padding(p, p, p, p)
    }
    pub fn margin(mut self, t: f32, r: f32, b: f32, l: f32) -> Self {
        self.spacing.get_or_insert_with(Default::default).margin = [t, r, b, l];
        self
    }
    pub fn margin_all(self, m: f32) -> Self {
        self.margin(m, m, m, m)
    }

    // -- flex --------------------------------------------------------------

    pub fn flex_direction(mut self, d: FlexDir) -> Self {
        self.flex.get_or_insert_with(Default::default).direction = d;
        self
    }
    pub fn flex_wrap(mut self, w: FlexWrap) -> Self {
        self.flex.get_or_insert_with(Default::default).wrap = w;
        self
    }
    pub fn justify_content(mut self, j: Justify) -> Self {
        self.flex.get_or_insert_with(Default::default).justify = j;
        self
    }
    pub fn align_items(mut self, a: Align) -> Self {
        self.flex.get_or_insert_with(Default::default).align_items = a;
        self
    }
    pub fn align_self(mut self, a: Align) -> Self {
        self.flex.get_or_insert_with(Default::default).align_self = a;
        self
    }
    pub fn gap(mut self, g: f32) -> Self {
        self.flex.get_or_insert_with(Default::default).gap = g;
        self
    }
    pub fn flex_grow(mut self, g: f32) -> Self {
        self.flex.get_or_insert_with(Default::default).grow = g;
        self
    }
    pub fn flex_shrink(mut self, s: f32) -> Self {
        self.flex.get_or_insert_with(Default::default).shrink = s;
        self
    }
    pub fn flex_basis(mut self, b: Dimension) -> Self {
        self.flex.get_or_insert_with(Default::default).basis = b;
        self
    }

    // -- background --------------------------------------------------------

    pub fn background_color(mut self, c: Color) -> Self {
        self.background.get_or_insert_with(Default::default).color = c.to_rgba();
        self
    }

    // -- border ------------------------------------------------------------

    pub fn border_width(mut self, w: f32) -> Self {
        self.border.get_or_insert_with(Default::default).width = w;
        self
    }
    pub fn border_color(mut self, c: Color) -> Self {
        self.border.get_or_insert_with(Default::default).color = c.to_rgba();
        self
    }
    pub fn border_radius(mut self, r: f32) -> Self {
        self.border.get_or_insert_with(Default::default).radius = [r; 4];
        self
    }
    pub fn border_radius_each(mut self, tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        self.border.get_or_insert_with(Default::default).radius = [tl, tr, br, bl];
        self
    }

    // -- text style --------------------------------------------------------

    pub fn font_size(mut self, s: f32) -> Self {
        self.text_style
            .get_or_insert_with(Default::default)
            .font_size = s;
        self
    }
    pub fn font_weight(mut self, w: u16) -> Self {
        self.text_style
            .get_or_insert_with(Default::default)
            .font_weight = w;
        self
    }
    pub fn color(mut self, c: Color) -> Self {
        self.text_style.get_or_insert_with(Default::default).color = c.to_rgba();
        self
    }
    pub fn line_height(mut self, h: f32) -> Self {
        self.text_style
            .get_or_insert_with(Default::default)
            .line_height = h;
        self
    }
    pub fn text_align(mut self, a: TextAlign) -> Self {
        self.text_style
            .get_or_insert_with(Default::default)
            .text_align = a;
        self
    }

    // -- position ----------------------------------------------------------

    pub fn position(mut self, p: Position) -> Self {
        self.position.get_or_insert_with(Default::default).position = p;
        self
    }
    pub fn top(mut self, d: Dimension) -> Self {
        self.position.get_or_insert_with(Default::default).top = d;
        self
    }
    pub fn left(mut self, d: Dimension) -> Self {
        self.position.get_or_insert_with(Default::default).left = d;
        self
    }
    pub fn right(mut self, d: Dimension) -> Self {
        self.position.get_or_insert_with(Default::default).right = d;
        self
    }
    pub fn bottom(mut self, d: Dimension) -> Self {
        self.position.get_or_insert_with(Default::default).bottom = d;
        self
    }
    pub fn z_index(mut self, z: i32) -> Self {
        self.position.get_or_insert_with(Default::default).z_index = z;
        self
    }

    // -- effects -----------------------------------------------------------

    pub fn opacity(mut self, o: f32) -> Self {
        self.effect.get_or_insert_with(Default::default).opacity = o;
        self
    }
    pub fn box_shadow(mut self, s: BoxShadow) -> Self {
        self.effect.get_or_insert_with(Default::default).box_shadow = Some(s);
        self
    }
    pub fn transform(mut self, t: Transform) -> Self {
        self.effect.get_or_insert_with(Default::default).transform = Some(t);
        self
    }

    // -- overflow ----------------------------------------------------------

    pub fn overflow_x(mut self, o: Overflow) -> Self {
        self.overflow
            .get_or_insert_with(Default::default)
            .overflow_x = o;
        self
    }
    pub fn overflow_y(mut self, o: Overflow) -> Self {
        self.overflow
            .get_or_insert_with(Default::default)
            .overflow_y = o;
        self
    }
    pub fn overflow(mut self, o: Overflow) -> Self {
        let ov = self.overflow.get_or_insert_with(Default::default);
        ov.overflow_x = o;
        ov.overflow_y = o;
        self
    }
    pub fn visibility(mut self, v: bool) -> Self {
        self.overflow
            .get_or_insert_with(Default::default)
            .visibility = v;
        self
    }
}

#[macro_export]
macro_rules! style_apply {
    ($s:expr, padding, $v:expr) => {
        $s.padding_all($v)
    };
    ($s:expr, margin, $v:expr) => {
        $s.margin_all($v)
    };
    ($s:expr, gap, $v:expr) => {
        $s.gap($v)
    };
    ($s:expr, width, $v:expr) => {
        $s.width($v)
    };
    ($s:expr, height, $v:expr) => {
        $s.height($v)
    };
    ($s:expr, background_color, $v:expr) => {
        $s.background_color($v)
    };
    ($s:expr, border_radius, $v:expr) => {
        $s.border_radius($v)
    };
    ($s:expr, border_width, $v:expr) => {
        $s.border_width($v)
    };
    ($s:expr, border_color, $v:expr) => {
        $s.border_color($v)
    };
    ($s:expr, flex_direction, $v:expr) => {
        $s.flex_direction($v)
    };
    ($s:expr, justify_content, $v:expr) => {
        $s.justify_content($v)
    };
    ($s:expr, align_items, $v:expr) => {
        $s.align_items($v)
    };
    ($s:expr, font_size, $v:expr) => {
        $s.font_size($v)
    };
    ($s:expr, color, $v:expr) => {
        $s.color($v)
    };
    ($s:expr, opacity, $v:expr) => {
        $s.opacity($v)
    };
    ($s:expr, box_shadow, $v:expr) => {
        $s.box_shadow($v)
    };
    ($s:expr, display, $v:expr) => {
        $s.display($v)
    };
    ($s:expr, overflow, $v:expr) => {
        $s.overflow($v)
    };
    ($s:expr, position, $v:expr) => {
        $s.position($v)
    };
    ($s:expr, top, $v:expr) => {
        $s.top($v)
    };
    ($s:expr, left, $v:expr) => {
        $s.left($v)
    };
    ($s:expr, right, $v:expr) => {
        $s.right($v)
    };
    ($s:expr, bottom, $v:expr) => {
        $s.bottom($v)
    };
    ($s:expr, z_index, $v:expr) => {
        $s.z_index($v)
    };
    ($s:expr, overflow_x, $v:expr) => {
        $s.overflow_x($v)
    };
    ($s:expr, overflow_y, $v:expr) => {
        $s.overflow_y($v)
    };
    ($s:expr, visibility, $v:expr) => {
        $s.visibility($v)
    };
    ($s:expr, line_height, $v:expr) => {
        $s.line_height($v)
    };
}

#[macro_export]
macro_rules! style {
    ($($key:ident : $val:expr),* $(,)?) => {{
        let mut __s = $crate::Style::new();
        $(
            __s = $crate::style_apply!(__s, $key, $val);
        )*
        __s
    }};
}
