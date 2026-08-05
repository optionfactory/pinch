use crate::config::{LayoutBlock, LayoutDirection, LayoutEdge};
use crate::processes::panes::ProcessPane;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTarget {
    Process(usize),
    CombinedLogs,
}

#[derive(Debug, Clone)]
pub struct PaneGeometry {
    pub target: PaneTarget,
    pub area: Rect,
    pub toggle_area: Rect,
    pub restart_area: Rect,
    pub wrap_area: Rect,
    pub zoom_area: Rect,
    pub link_area: Rect,
}

impl PaneGeometry {
    pub fn new(target: PaneTarget, area: Rect) -> Self {
        let base_x = area.x + 1;
        let base_y = area.y;
        Self {
            target,
            area,
            toggle_area: safe_rect(base_x, base_y, 4, area),
            restart_area: safe_rect(base_x + 4, base_y, 4, area),
            wrap_area: safe_rect(base_x + 8, base_y, 4, area),
            zoom_area: safe_rect(base_x + 12, base_y, 4, area),
            link_area: safe_rect(base_x + 16, base_y, 4, area),
        }
    }
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
        geometries.push(PaneGeometry::new(PaneTarget::Process(zoom_id), grid_area));
        return geometries;
    }

    let mut remaining_area = grid_area;
    let mut assigned_panes = std::collections::HashSet::new();
    let mut include_combined_logs = false;
    let mut unassigned_container: Option<Rect> = None;

    for item in layout_items {
        let percentage = item.size.min(100);

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
            geometries.push(PaneGeometry::new(target, area));
        };

        if let Some(ref sub_splits) = item.splits {
            let split_direction = match item.direction {
                Some(LayoutDirection::Horizontal) => Direction::Horizontal,
                Some(LayoutDirection::Vertical) => Direction::Vertical,
                _ => match item.edge {
                    LayoutEdge::Left | LayoutEdge::Right => Direction::Vertical,
                    LayoutEdge::Top | LayoutEdge::Bottom => Direction::Horizontal,
                },
            };

            let constraints: Vec<Constraint> = sub_splits
                .iter()
                .map(|s| Constraint::Percentage(s.size))
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

                if sub_item.unassigned.unwrap_or(false) {
                    unassigned_container = Some(sub_area);
                    continue;
                }

                let target = if let Some(name) = &sub_item.name {
                    if name == "combined-logs" {
                        if include_combined_logs {
                            continue;
                        }
                        include_combined_logs = true;
                        Some(PaneTarget::CombinedLogs)
                    } else if let Some(pane) = panes.iter().find(|p| p.config.name == *name) {
                        if assigned_panes.contains(&pane.id) {
                            continue;
                        }
                        assigned_panes.insert(pane.id);
                        Some(PaneTarget::Process(pane.id))
                    } else {
                        None
                    }
                } else {
                    None
                };

                if let Some(t) = target {
                    register_geo(t, sub_area, &mut geometries);
                }
            }
        } else if item.unassigned.unwrap_or(false) {
            unassigned_container = Some(carved_area);
        } else if let Some(ref name) = item.name {
            let target = if name == "combined-logs" {
                if include_combined_logs {
                    continue;
                }
                include_combined_logs = true;
                Some(PaneTarget::CombinedLogs)
            } else if let Some(pane) = panes.iter().find(|p| p.config.name == *name) {
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
        let target_area = unassigned_container.unwrap_or(remaining_area);
        if target_area.width > 0 && target_area.height > 0 {
            let num_panes = unassigned_panes.len();

            // If the target area is wider than it is tall (e.g. bottom unassigned bar),
            // arrange panes horizontally side-by-side instead of stacking vertically!
            if target_area.width >= target_area.height * 2 {
                let col_areas = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Ratio(1, num_panes as u32); num_panes])
                    .split(target_area);

                for (idx, pane) in unassigned_panes.iter().enumerate() {
                    let area = col_areas[idx];
                    geometries.push(PaneGeometry::new(PaneTarget::Process(pane.id), area));
                }
            } else {
                let max_per_col = 4;
                let num_cols = num_panes.div_ceil(max_per_col);

                let column_areas = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(vec![Constraint::Ratio(1, num_cols as u32); num_cols])
                    .split(target_area);

                for col_idx in 0..num_cols {
                    let start_idx = col_idx * max_per_col;
                    let end_idx = std::cmp::min(start_idx + max_per_col, num_panes);
                    let items_in_col = end_idx - start_idx;

                    let row_areas = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(vec![Constraint::Ratio(1, items_in_col as u32); items_in_col])
                        .split(column_areas[col_idx]);

                    for row_idx in 0..items_in_col {
                        let pane = unassigned_panes[start_idx + row_idx];
                        let area = row_areas[row_idx];
                        geometries.push(PaneGeometry::new(PaneTarget::Process(pane.id), area));
                    }
                }
            }
        }
    }

    geometries
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
