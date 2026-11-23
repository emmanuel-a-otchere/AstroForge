//! Node canvas geometry — pure layout + hit-testing for the linear
//! pipeline column (port of `src/lib/node-canvas-logic.ts`).
//!
//! The pipeline graph is a LINEAR sequence (bounded ~20 stages), so
//! the default layout is a vertical column and wires connect
//! consecutive stages. Same constants as the web canvas
//! (`NODE_WIDTH=170`, `NODE_HEIGHT=52`, `ROW_GAP=34`) so side-by-side
//! comparison stays honest. All functions are pure and unit-tested.

/// Card width in points (matches the web canvas).
pub const NODE_WIDTH: f32 = 170.0;
/// Card height in points (matches the web canvas).
pub const NODE_HEIGHT: f32 = 52.0;
/// Vertical gap between cards (matches the web canvas).
pub const ROW_GAP: f32 = 34.0;

/// Top-left position of the card at `index` (0-based) inside the
/// column, relative to the canvas origin.
pub fn card_position(index: usize) -> egui::Vec2 {
    egui::vec2(0.0, index as f32 * (NODE_HEIGHT + ROW_GAP))
}

/// Total column size for `count` cards.
pub fn column_size(count: usize) -> egui::Vec2 {
    if count == 0 {
        return egui::vec2(NODE_WIDTH, 0.0);
    }
    egui::vec2(
        NODE_WIDTH,
        count as f32 * NODE_HEIGHT + (count - 1) as f32 * ROW_GAP,
    )
}

/// Hit-test a canvas-local position against the card column.
/// Positions in the horizontal gutters do not hit; positions in the
/// vertical gaps between cards do not hit either.
pub fn hit_test(count: usize, pos: egui::Vec2) -> Option<usize> {
    if count == 0 || pos.x < 0.0 || pos.x > NODE_WIDTH {
        return None;
    }
    let row = (pos.y / (NODE_HEIGHT + ROW_GAP)).floor() as usize;
    if row >= count {
        return None;
    }
    let local_y = pos.y - row as f32 * (NODE_HEIGHT + ROW_GAP);
    if local_y <= NODE_HEIGHT {
        Some(row)
    } else {
        None
    }
}

/// Where a dragged card would land: the target index its center-Y
/// overlaps, clamped to [0, count). Returns None when the pointer is
/// outside the column's vertical extent entirely.
pub fn drop_index(count: usize, pointer_y: f32) -> Option<usize> {
    if count == 0 || pointer_y < 0.0 {
        return None;
    }
    let column_h = column_size(count).y;
    if pointer_y > column_h {
        return None;
    }
    // Snap to the nearest card slot (center-weighted).
    let idx = ((pointer_y + NODE_HEIGHT / 2.0) / (NODE_HEIGHT + ROW_GAP)).floor() as usize;
    Some(idx.min(count - 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_a_vertical_column_with_gaps() {
        assert_eq!(card_position(0), egui::vec2(0.0, 0.0));
        assert_eq!(
            card_position(2),
            egui::vec2(0.0, 2.0 * (NODE_HEIGHT + ROW_GAP))
        );
        assert_eq!(
            column_size(3),
            egui::vec2(NODE_WIDTH, 3.0 * NODE_HEIGHT + 2.0 * ROW_GAP)
        );
        assert_eq!(column_size(0), egui::vec2(NODE_WIDTH, 0.0));
    }

    #[test]
    fn hit_test_finds_cards_not_gaps() {
        assert_eq!(hit_test(3, egui::vec2(85.0, 10.0)), Some(0));
        assert_eq!(
            hit_test(3, egui::vec2(85.0, NODE_HEIGHT + ROW_GAP + 5.0)),
            Some(1)
        );
        // In the gap between cards: no hit.
        assert_eq!(hit_test(3, egui::vec2(85.0, NODE_HEIGHT + 5.0)), None);
        // Outside the column horizontally: no hit.
        assert_eq!(hit_test(3, egui::vec2(NODE_WIDTH + 10.0, 10.0)), None);
        assert_eq!(hit_test(0, egui::vec2(10.0, 10.0)), None);
    }

    #[test]
    fn drop_index_snaps_to_nearest_slot() {
        assert_eq!(drop_index(3, 0.0), Some(0));
        assert_eq!(drop_index(3, NODE_HEIGHT + ROW_GAP + 10.0), Some(1));
        assert_eq!(drop_index(3, 999.0), None);
        assert_eq!(drop_index(0, 10.0), None);
    }
}
