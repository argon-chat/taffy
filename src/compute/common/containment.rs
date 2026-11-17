//! Size containment (`contain: size` / `inline-size`, and the `container-type` that implies it)
use crate::geometry::Size;
use crate::style::{BoxSizing, Contain, CoreStyle};
use crate::tree::{LayoutInput, LayoutPartialTree, LayoutPartialTreeExt as _, SizingMode};
use crate::util::MaybeMath;
use crate::util::{MaybeResolve, ResolveOrZero};

/// `inputs` with each size-contained axis the parent left open fixed at the box's size as if it
/// were empty: its padding and border, clamped by its own `min-*`/`max-*`. An axis its own style
/// sizes is left to the layout algorithm.
pub(crate) fn size_contained(
    tree: &impl LayoutPartialTree,
    style: &impl CoreStyle,
    inputs: LayoutInput,
) -> LayoutInput {
    let contain = style.contain();
    let open = Size {
        width: contain.contains(Contain::INLINE_SIZE) && inputs.known_dimensions.width.is_none(),
        height: contain.contains(Contain::BLOCK_SIZE) && inputs.known_dimensions.height.is_none(),
    };
    if !open.width && !open.height {
        return inputs;
    }
    let parent_size = inputs.parent_size;
    let calc = |val, basis| tree.calc(val, basis);
    let padding = style.padding().resolve_or_zero(parent_size.width, calc);
    let border = style.border().resolve_or_zero(parent_size.width, calc);
    let padding_border = padding.sum_axes() + border.sum_axes();
    let empty = if inputs.sizing_mode == SizingMode::InherentSize {
        let adjustment = if style.box_sizing() == BoxSizing::ContentBox { padding_border } else { Size::ZERO };
        let aspect_ratio = style.aspect_ratio();
        let own = style.size().maybe_resolve(parent_size, calc).maybe_apply_aspect_ratio(aspect_ratio);
        if (!open.width || own.width.is_some()) && (!open.height || own.height.is_some()) {
            return inputs;
        }
        let min = style.min_size().maybe_resolve(parent_size, calc).maybe_add(adjustment);
        let max = style.max_size().maybe_resolve(parent_size, calc).maybe_add(adjustment);
        Size {
            width: own.width.maybe_add(adjustment.width).unwrap_or(padding_border.width),
            height: own.height.maybe_add(adjustment.height).unwrap_or(padding_border.height),
        }
        .maybe_clamp(min, max)
        .zip_map(padding_border, f32::max)
    } else {
        padding_border
    };
    let known = Size {
        width: if open.width { Some(empty.width) } else { inputs.known_dimensions.width },
        height: if open.height { Some(empty.height) } else { inputs.known_dimensions.height },
    };
    let definite = Size {
        width: open.width || inputs.known_dimensions_are_definite.width,
        height: open.height || inputs.known_dimensions_are_definite.height,
    };
    LayoutInput { known_dimensions: known, known_dimensions_are_definite: definite, ..inputs }
}

#[cfg(all(test, feature = "taffy_tree", feature = "block_layout", feature = "flexbox"))]
mod tests {
    use crate::prelude::*;
    use crate::Contain;

    fn content(t: &mut TaffyTree<()>) -> NodeId {
        t.new_leaf(Style { size: Size { width: length(300.0), height: length(40.0) }, ..Default::default() }).unwrap()
    }

    /// The box's size, as a row flex item that does not grow (so its width is its content's), in a
    /// 1000x500 row.
    fn in_row(contain: Contain, style: Style) -> Size<f32> {
        let mut t: TaffyTree<()> = TaffyTree::new();
        let inner = content(&mut t);
        let item = t.new_with_children(Style { contain, padding: length(5.0), ..style }, &[inner]).unwrap();
        let row = t
            .new_with_children(
                Style {
                    display: Display::Flex,
                    align_items: AlignItems::START,
                    size: Size { width: length(1000.0), height: length(500.0) },
                    ..Default::default()
                },
                &[item],
            )
            .unwrap();
        t.compute_layout(row, Size::MAX_CONTENT).unwrap();
        t.layout(item).unwrap().size
    }

    #[test]
    fn an_inline_size_contained_box_is_as_wide_as_if_empty() {
        assert_eq!(in_row(Contain::NONE, Style::default()), Size { width: 310.0, height: 50.0 });
        assert_eq!(in_row(Contain::INLINE_SIZE, Style::default()), Size { width: 10.0, height: 50.0 });
        assert_eq!(in_row(Contain::SIZE, Style::default()), Size { width: 10.0, height: 10.0 });
    }

    #[test]
    fn its_own_size_and_min_size_still_apply() {
        let sized = Style { size: Size { width: length(200.0), height: auto() }, ..Default::default() };
        assert_eq!(in_row(Contain::INLINE_SIZE, sized).width, 200.0);
        let floored = Style { min_size: Size { width: length(120.0), height: auto() }, ..Default::default() };
        assert_eq!(in_row(Contain::INLINE_SIZE, floored).width, 120.0);
        let grows = Style { flex_grow: 1.0, ..Default::default() };
        assert_eq!(in_row(Contain::INLINE_SIZE, grows).width, 1000.0, "the parent still sizes it");
    }

    #[test]
    fn a_block_child_still_stretches_to_its_parent() {
        let mut t: TaffyTree<()> = TaffyTree::new();
        let inner = content(&mut t);
        let item =
            t.new_with_children(Style { contain: Contain::INLINE_SIZE, ..Default::default() }, &[inner]).unwrap();
        let block = t
            .new_with_children(
                Style {
                    display: Display::Block,
                    size: Size { width: length(640.0), height: auto() },
                    ..Default::default()
                },
                &[item],
            )
            .unwrap();
        t.compute_layout(block, Size::MAX_CONTENT).unwrap();
        assert_eq!(t.layout(item).unwrap().size, Size { width: 640.0, height: 40.0 });
    }
}
