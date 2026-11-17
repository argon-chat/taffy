//! css-sizing-4 §5.1: a block whose height comes from `aspect-ratio` grows to fit taller content
//! under `min-height: auto`, as Chrome lays it out; a scroll container or `min-height: 0` keeps
//! the ratio.
#[cfg(test)]
mod aspect_ratio_min_content {
    use taffy::prelude::*;
    use taffy::{Overflow, Point, TaffyTree};

    fn height_of(ratio_box: Style) -> f32 {
        let mut taffy: TaffyTree<()> = TaffyTree::new();
        let content =
            taffy.new_leaf(Style { size: Size { width: auto(), height: length(80.0) }, ..Default::default() }).unwrap();
        let ratio_box = taffy.new_with_children(ratio_box, &[content]).unwrap();
        let root = taffy
            .new_with_children(
                Style {
                    display: Display::Block,
                    size: Size { width: length(200.0), height: auto() },
                    ..Default::default()
                },
                &[ratio_box],
            )
            .unwrap();
        taffy.compute_layout(root, Size::MAX_CONTENT).unwrap();
        taffy.layout(ratio_box).unwrap().size.height
    }

    fn ratio_box(width: Dimension) -> Style {
        Style {
            display: Display::Block,
            size: Size { width, height: auto() },
            aspect_ratio: Some(2.0),
            ..Default::default()
        }
    }

    #[test]
    fn taller_content_grows_a_fixed_width_box() {
        assert_eq!(height_of(ratio_box(length(100.0))), 80.0);
    }

    #[test]
    fn taller_content_grows_a_stretched_box_only_past_its_ratio() {
        assert_eq!(height_of(ratio_box(auto())), 100.0);
    }

    #[test]
    fn a_scroll_container_keeps_the_ratio() {
        let style = Style { overflow: Point { x: Overflow::Hidden, y: Overflow::Hidden }, ..ratio_box(length(100.0)) };
        assert_eq!(height_of(style), 50.0);
    }

    #[test]
    fn a_zero_min_height_keeps_the_ratio() {
        let style = Style { min_size: Size { width: auto(), height: length(0.0) }, ..ratio_box(length(100.0)) };
        assert_eq!(height_of(style), 50.0);
    }
}
