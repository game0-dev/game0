use std::collections::HashMap;

use taffy::prelude::*;

use crate::render_tree::node::{Display as GDisplay, ElementTag, NodeId, StyleFlags};
use crate::render_tree::RenderTree;
use crate::style::groups::TextStyleGroup;
use crate::style::types::{
    Align as GAlign,
    Dimension as GDimension,
    FlexDir as GFlexDir,
    FlexWrap as GFlexWrap,
    Justify as GJustify,
    Position as GPosition,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct LayoutRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub fn compute_layout(
    tree: &mut RenderTree,
    root: NodeId,
    viewport_w: f32,
    viewport_h: f32,
) -> Result<(), String> {
    let mut taffy = taffy::TaffyTree::new();
    let mut node_map = HashMap::<NodeId, taffy::NodeId>::new();

    let taffy_root = build_taffy_subtree(tree, &mut taffy, &mut node_map, root)?;
    taffy
        .compute_layout(
            taffy_root,
            Size {
                width: AvailableSpace::Definite(viewport_w),
                height: AvailableSpace::Definite(viewport_h),
            },
        )
        .map_err(|e| format!("taffy compute_layout failed: {e}"))?;

    read_back_layout(tree, &taffy, &node_map, root, 0.0, 0.0)?;
    Ok(())
}

pub fn render_print(tree: &RenderTree, root: NodeId, viewport_w: f32, viewport_h: f32) {
    println!("=== Render Output ({viewport_w:.0} x {viewport_h:.0}) ===");
    render_print_node(tree, root, 0);
}

fn build_taffy_subtree(
    tree: &RenderTree,
    taffy: &mut taffy::TaffyTree,
    node_map: &mut HashMap<NodeId, taffy::NodeId>,
    id: NodeId,
) -> Result<taffy::NodeId, String> {
    let node = tree
        .get(id)
        .ok_or_else(|| format!("missing render node: {:?}", id))?;
    let children = node.children.to_vec();

    let mut taffy_children = Vec::with_capacity(children.len());
    for child in children {
        let child_id = build_taffy_subtree(tree, taffy, node_map, child)?;
        taffy_children.push(child_id);
    }

    let style = to_taffy_style(tree, id);
    let taffy_id = if taffy_children.is_empty() {
        taffy
            .new_leaf(style)
            .map_err(|e| format!("taffy new_leaf failed: {e}"))?
    } else {
        taffy
            .new_with_children(style, &taffy_children)
            .map_err(|e| format!("taffy new_with_children failed: {e}"))?
    };

    node_map.insert(id, taffy_id);
    Ok(taffy_id)
}

fn read_back_layout(
    tree: &mut RenderTree,
    taffy: &taffy::TaffyTree,
    node_map: &HashMap<NodeId, taffy::NodeId>,
    id: NodeId,
    parent_x: f32,
    parent_y: f32,
) -> Result<(), String> {
    let node = tree
        .get(id)
        .ok_or_else(|| format!("missing render node: {:?}", id))?;
    let children = node.children.to_vec();

    let taffy_id = *node_map
        .get(&id)
        .ok_or_else(|| format!("taffy id not found for {:?}", id))?;
    let layout = taffy
        .layout(taffy_id)
        .map_err(|e| format!("taffy layout lookup failed: {e}"))?;

    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;
    tree.set_layout_rect(
        id,
        LayoutRect {
            x: abs_x,
            y: abs_y,
            width: layout.size.width,
            height: layout.size.height,
        },
    );

    for child in children {
        read_back_layout(tree, taffy, node_map, child, abs_x, abs_y)?;
    }
    Ok(())
}

fn render_print_node(tree: &RenderTree, id: NodeId, depth: usize) {
    let Some(node) = tree.get(id) else { return };
    let indent = "  ".repeat(depth);
    let rect = tree.get_layout_rect(id).unwrap_or_default();

    let mut extras = Vec::new();
    if node.style_flags.contains(StyleFlags::SIZE) {
        extras.push("size");
    }
    if node.style_flags.contains(StyleFlags::SPACING) {
        extras.push("spacing");
    }
    if node.style_flags.contains(StyleFlags::FLEX) {
        extras.push("flex");
    }
    if node.style_flags.contains(StyleFlags::BACKGROUND) {
        extras.push("bg");
    }
    if node.style_flags.contains(StyleFlags::BORDER) {
        extras.push("border");
    }

    let style_info = if extras.is_empty() {
        String::new()
    } else {
        format!(" [{}]", extras.join(","))
    };

    let mut content_info = String::new();
    if let Some(text) = tree.get_text(id) {
        if !text.is_empty() {
            content_info = format!(" \"{}\"", text);
        }
    }

    println!(
        "{indent}<{:?}> x={:.1} y={:.1} w={:.1} h={:.1}{}{}",
        node.tag, rect.x, rect.y, rect.width, rect.height, content_info, style_info
    );

    for &child in &node.children {
        render_print_node(tree, child, depth + 1);
    }
}

fn to_taffy_style(tree: &RenderTree, id: NodeId) -> taffy::Style {
    let mut style = taffy::Style::default();

    if let Some(node) = tree.get(id) {
        style.display = match node.display {
            GDisplay::None => Display::None,
            GDisplay::Flex | GDisplay::Block => Display::Flex,
        };
    }

    if let Some(size) = tree.size_styles.get(id) {
        style.size.width = map_dimension(size.width);
        style.size.height = map_dimension(size.height);
        style.min_size.width = map_dimension(size.min_width);
        style.min_size.height = map_dimension(size.min_height);
        style.max_size.width = map_dimension(size.max_width);
        style.max_size.height = map_dimension(size.max_height);
    }

    if let Some(space) = tree.spacing_styles.get(id) {
        style.padding = Rect {
            left: LengthPercentage::length(space.padding[3]),
            right: LengthPercentage::length(space.padding[1]),
            top: LengthPercentage::length(space.padding[0]),
            bottom: LengthPercentage::length(space.padding[2]),
        };
        style.margin = Rect {
            left: LengthPercentageAuto::length(space.margin[3]),
            right: LengthPercentageAuto::length(space.margin[1]),
            top: LengthPercentageAuto::length(space.margin[0]),
            bottom: LengthPercentageAuto::length(space.margin[2]),
        };
    }

    if let Some(flex) = tree.flex_styles.get(id) {
        style.flex_direction = map_flex_direction(flex.direction);
        style.flex_wrap = map_flex_wrap(flex.wrap);
        style.justify_content = Some(map_justify(flex.justify));
        style.align_items = Some(map_align(flex.align_items));
        style.align_self = Some(map_align(flex.align_self));
        style.gap = Size {
            width: LengthPercentage::length(flex.gap),
            height: LengthPercentage::length(flex.gap),
        };
        style.flex_grow = flex.grow;
        style.flex_shrink = flex.shrink;
        style.flex_basis = map_dimension(flex.basis);
    }

    if let Some(pos) = tree.position_styles.get(id) {
        style.position = map_position(pos.position);
        style.inset = Rect {
            left: map_inset(pos.left),
            right: map_inset(pos.right),
            top: map_inset(pos.top),
            bottom: map_inset(pos.bottom),
        };
    }

    // Leaf nodes without explicit size still need a sensible intrinsic size
    // to avoid all-zero layouts in the println pass.
    apply_intrinsic_leaf_size(tree, id, &mut style);

    style
}

fn apply_intrinsic_leaf_size(tree: &RenderTree, id: NodeId, style: &mut taffy::Style) {
    let Some(node) = tree.get(id) else { return };
    if !node.children.is_empty() {
        return;
    }

    match node.tag {
        ElementTag::Text | ElementTag::Icon => {
            let text = tree.get_text(id).unwrap_or("");
            let text_style = tree
                .text_style_groups
                .get(id)
                .cloned()
                .unwrap_or_else(TextStyleGroup::default);
            let h = text_style.font_size * text_style.line_height.max(1.0);
            let w = text.len() as f32 * text_style.font_size * 0.55;

            if style.size.width == taffy::Dimension::auto() {
                style.size.width = taffy::Dimension::length(w.max(1.0));
            }
            if style.size.height == taffy::Dimension::auto() {
                style.size.height = taffy::Dimension::length(h.max(1.0));
            }
        }
        ElementTag::Image => {
            if style.size.width == taffy::Dimension::auto() {
                style.size.width = taffy::Dimension::length(100.0);
            }
            if style.size.height == taffy::Dimension::auto() {
                style.size.height = taffy::Dimension::length(100.0);
            }
        }
        ElementTag::Input => {
            if style.size.width == taffy::Dimension::auto() {
                style.size.width = taffy::Dimension::length(240.0);
            }
            if style.size.height == taffy::Dimension::auto() {
                style.size.height = taffy::Dimension::length(32.0);
            }
        }
        _ => {}
    }
}

fn map_dimension(v: GDimension) -> taffy::Dimension {
    match v {
        GDimension::Auto => taffy::Dimension::auto(),
        GDimension::Px(px) => taffy::Dimension::length(px),
        GDimension::Percent(p) => taffy::Dimension::percent((p / 100.0).clamp(0.0, 1.0)),
    }
}

fn map_flex_direction(v: GFlexDir) -> taffy::FlexDirection {
    match v {
        GFlexDir::Row => FlexDirection::Row,
        GFlexDir::Column => FlexDirection::Column,
        GFlexDir::RowReverse => FlexDirection::RowReverse,
        GFlexDir::ColumnReverse => FlexDirection::ColumnReverse,
    }
}

fn map_flex_wrap(v: GFlexWrap) -> taffy::FlexWrap {
    match v {
        GFlexWrap::NoWrap => FlexWrap::NoWrap,
        GFlexWrap::Wrap => FlexWrap::Wrap,
    }
}

fn map_justify(v: GJustify) -> taffy::JustifyContent {
    match v {
        GJustify::Start => JustifyContent::FlexStart,
        GJustify::Center => JustifyContent::Center,
        GJustify::End => JustifyContent::FlexEnd,
        GJustify::SpaceBetween => JustifyContent::SpaceBetween,
        GJustify::SpaceAround => JustifyContent::SpaceAround,
    }
}

fn map_align(v: GAlign) -> taffy::AlignItems {
    match v {
        GAlign::Stretch => AlignItems::Stretch,
        GAlign::Start => AlignItems::FlexStart,
        GAlign::Center => AlignItems::Center,
        GAlign::End => AlignItems::FlexEnd,
    }
}

fn map_position(v: GPosition) -> taffy::Position {
    match v {
        GPosition::Relative => taffy::Position::Relative,
        GPosition::Absolute | GPosition::Fixed => taffy::Position::Absolute,
    }
}

fn map_inset(v: GDimension) -> taffy::LengthPercentageAuto {
    match v {
        GDimension::Auto => taffy::LengthPercentageAuto::auto(),
        GDimension::Px(px) => taffy::LengthPercentageAuto::length(px),
        GDimension::Percent(p) => taffy::LengthPercentageAuto::percent((p / 100.0).clamp(0.0, 1.0)),
    }
}
