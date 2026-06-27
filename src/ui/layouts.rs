use crate::config::{LayoutBlock, LayoutEdge};
use crate::process::panes::ProcessPane;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct Viewport {
    pub start: usize,
    pub end: usize,
    pub is_scrolled: bool,
}

impl Viewport {
    pub fn visible_range(total_items: usize, available_height: usize, top_index: Option<usize>) -> Self {
        match top_index {
            None => {
                let start = total_items.saturating_sub(available_height);
                Self {
                    start,
                    end: total_items,
                    is_scrolled: false,
                }
            }
            Some(top_idx) => {
                let clamped_top = top_idx.min(total_items.saturating_sub(available_height));
                let end = (clamped_top + available_height).min(total_items);
                Self {
                    start: clamped_top,
                    end,
                    is_scrolled: true,
                }
            }
        }
    }
}

fn safe_rect(x: u16, y: u16, width: u16, pane_area: Rect) -> Rect {
    let max_x = pane_area.x + pane_area.width;
    let fits = x < max_x;

    let w = if fits { width.min(max_x.saturating_sub(x)) } else { 0 };
    let h = if fits { 1 } else { 0 };

    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTarget {
    Process(usize),
    CombinedLogs,
}

pub struct PaneGeometry {
    pub target: PaneTarget,
    pub area: Rect,
    pub toggle_area: Rect,
    pub restart_area: Rect,
    pub wrap_area: Rect,
    pub zoom_area: Rect,
}

pub fn compute_pane_geometries(
    grid_area: Rect,
    panes: &[ProcessPane],
    zoomed_pane: Option<usize>,
    layout_items: &[LayoutBlock],
) -> Vec<PaneGeometry> {
    let mut geometries = vec![];
    if panes.is_empty() {
        return geometries;
    }

    if let Some(zoom_id) = zoomed_pane {
        let area = grid_area;
        let base_x = area.x;
        let base_y = area.y;
        geometries.push(PaneGeometry {
            target: PaneTarget::Process(zoom_id),
            area,
            toggle_area: safe_rect(base_x, base_y, 4, area),
            restart_area: safe_rect(base_x + 4, base_y, 4, area),
            wrap_area: safe_rect(base_x + 8, base_y, 4, area),
            zoom_area: safe_rect(base_x + 12, base_y, 4, area),
        });
        return geometries;
    }

    let mut remaining_area = grid_area;
    let mut assigned_panes = std::collections::HashSet::new();
    let mut include_combined_logs = false;

    for item in layout_items {
        let percentage = item.size_percentage.min(100);

        let edge_direction = match item.edge {
            LayoutEdge::Left | LayoutEdge::Right => Direction::Horizontal,
            LayoutEdge::Top | LayoutEdge::Bottom => Direction::Vertical,
        };

        let edge_constraints = match item.edge {
            LayoutEdge::Left | LayoutEdge::Top => [Constraint::Percentage(percentage), Constraint::Min(0)],
            LayoutEdge::Right | LayoutEdge::Bottom => [Constraint::Min(0), Constraint::Percentage(percentage)],
        };

        let chunks = Layout::default()
            .direction(edge_direction)
            .constraints(edge_constraints)
            .split(remaining_area);

        let carved_area = if item.edge == LayoutEdge::Left || item.edge == LayoutEdge::Top {
            remaining_area = chunks[1];
            chunks[0]
        } else {
            remaining_area = chunks[0];
            chunks[1]
        };

        let register_geo = |target: PaneTarget, area: Rect, geometries: &mut Vec<PaneGeometry>| {
            let base_x = area.x + 1;
            let base_y = area.y;
            geometries.push(PaneGeometry {
                target,
                area,
                toggle_area: safe_rect(base_x, base_y, 4, area),
                restart_area: safe_rect(base_x + 4, base_y, 4, area),
                wrap_area: safe_rect(base_x + 8, base_y, 4, area),
                zoom_area: safe_rect(base_x + 12, base_y, 4, area),
            });
        };

        if let Some(ref sub_splits) = item.splits {
            let split_direction = match item.direction.as_deref() {
                Some("horizontal") => Direction::Horizontal,
                Some("vertical") => Direction::Vertical,
                _ => match item.edge {
                    LayoutEdge::Left | LayoutEdge::Right => Direction::Vertical,
                    LayoutEdge::Top | LayoutEdge::Bottom => Direction::Horizontal,
                },
            };

            let constraints: Vec<Constraint> = sub_splits
                .iter()
                .map(|s| Constraint::Percentage(s.size_percentage))
                .collect();

            let sub_chunks = Layout::default()
                .direction(split_direction)
                .constraints(constraints)
                .split(carved_area);

            for (idx, sub_item) in sub_splits.iter().enumerate() {
                if idx >= sub_chunks.len() {
                    break;
                }
                let sub_area = sub_chunks[idx];

                let target = if sub_item.title == "Combined Logs" {
                    if include_combined_logs {
                        continue;
                    }
                    include_combined_logs = true;
                    Some(PaneTarget::CombinedLogs)
                } else if let Some(pane) = panes.iter().find(|p| p.config.title == sub_item.title) {
                    if assigned_panes.contains(&pane.id) {
                        continue;
                    }
                    assigned_panes.insert(pane.id);
                    Some(PaneTarget::Process(pane.id))
                } else {
                    None
                };

                if let Some(t) = target {
                    register_geo(t, sub_area, &mut geometries);
                }
            }
        } else if let Some(ref title) = item.title {
            let target = if title == "Combined Logs" {
                if include_combined_logs {
                    continue;
                }
                include_combined_logs = true;
                Some(PaneTarget::CombinedLogs)
            } else if let Some(pane) = panes.iter().find(|p| p.config.title == *title) {
                if assigned_panes.contains(&pane.id) {
                    continue;
                }
                assigned_panes.insert(pane.id);
                Some(PaneTarget::Process(pane.id))
            } else {
                None
            };

            if let Some(t) = target {
                register_geo(t, carved_area, &mut geometries);
            }
        }
    }

    let unassigned_panes: Vec<&ProcessPane> = panes.iter().filter(|p| !assigned_panes.contains(&p.id)).collect();

    if !unassigned_panes.is_empty() {
        let num_panes = unassigned_panes.len();
        let max_per_col = 4;
        let num_cols = num_panes.div_ceil(max_per_col);
        let column_areas = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(100 / num_cols as u16); num_cols])
            .split(remaining_area);

        for col_idx in 0..num_cols {
            let start_idx = col_idx * max_per_col;
            let end_idx = std::cmp::min(start_idx + max_per_col, num_panes);
            let items_in_col = end_idx - start_idx;

            let row_areas = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Percentage(100 / items_in_col as u16); items_in_col])
                .split(column_areas[col_idx]);

            for row_idx in 0..items_in_col {
                let pane = unassigned_panes[start_idx + row_idx];
                let area = row_areas[row_idx];
                let base_x = area.x + 1;
                let base_y = area.y;

                geometries.push(PaneGeometry {
                    target: PaneTarget::Process(pane.id),
                    area,
                    toggle_area: safe_rect(base_x, base_y, 4, area),
                    restart_area: safe_rect(base_x + 4, base_y, 4, area),
                    wrap_area: safe_rect(base_x + 8, base_y, 4, area),
                    zoom_area: safe_rect(base_x + 12, base_y, 4, area),
                });
            }
        }
    }

    geometries
}
