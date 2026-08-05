use crate::config::{LayoutDirection, LayoutEdge, LayoutNode};
use crate::processes::panes::ProcessPane;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use std::collections::HashSet;

/// Represents the target content rendered inside a layout pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneTarget {
    Process(usize),
    CombinedLogs,
}

/// The calculated screen coordinates and clickable header regions for a pane.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Constructs a new geometry with predefined header button positions
    /// clamped within the bounds of `area`.
    pub fn new(target: PaneTarget, area: Rect) -> Self {
        let base_x = area.x.saturating_add(1);
        let base_y = area.y;
        Self {
            target,
            area,
            toggle_area: Self::bounded_button_rect(base_x, base_y, 4, area),
            restart_area: Self::bounded_button_rect(base_x.saturating_add(4), base_y, 4, area),
            wrap_area: Self::bounded_button_rect(base_x.saturating_add(8), base_y, 4, area),
            zoom_area: Self::bounded_button_rect(base_x.saturating_add(12), base_y, 4, area),
            link_area: Self::bounded_button_rect(base_x.saturating_add(16), base_y, 4, area),
        }
    }

    /// Creates a 1-row-high button rectangle bounded within the parent pane's area,
    /// returning an empty rectangle if the button falls outside the visible width.
    fn bounded_button_rect(x: u16, y: u16, width: u16, pane_area: Rect) -> Rect {
        let max_x = pane_area.x.saturating_add(pane_area.width);
        let fits = x < max_x && pane_area.height > 0;
        let w = if fits { width.min(max_x.saturating_sub(x)) } else { 0 };
        let h = if fits { 1 } else { 0 };
        Rect::new(x, y, w, h)
    }
}

/// Tracks assigned panes and log state during a single layout resolution pass.
struct PaneResolver {
    assigned_panes: HashSet<usize>,
    include_combined_logs: bool,
}

impl PaneResolver {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            assigned_panes: HashSet::with_capacity(capacity),
            include_combined_logs: false,
        }
    }

    /// Resolves a configured pane name to a unique target, ensuring
    /// each process ID or combined log view is assigned at most once.
    fn resolve_target(&mut self, name: &str, panes: &[ProcessPane]) -> Option<PaneTarget> {
        if name == "combined-logs" {
            if self.include_combined_logs {
                return None;
            }
            self.include_combined_logs = true;
            return Some(PaneTarget::CombinedLogs);
        }
        let pane = panes.iter().find(|p| p.config.name == name)?;
        if self.assigned_panes.insert(pane.id) {
            Some(PaneTarget::Process(pane.id))
        } else {
            None
        }
    }
}

/// Computes the layout geometries for all panes according to the configured
/// recursive layout tree, handling zoom state and unassigned pane fallbacks.
pub fn compute_pane_geometries(
    grid_area: Rect,
    panes: &[ProcessPane],
    zoomed_pane: Option<usize>,
    layout_items: &[LayoutNode],
) -> Vec<PaneGeometry> {
    if panes.is_empty() {
        return Vec::new();
    }

    if let Some(zoom_id) = zoomed_pane.filter(|&id| panes.iter().any(|p| p.id == id)) {
        return vec![PaneGeometry::new(PaneTarget::Process(zoom_id), grid_area)];
    }

    let mut geometries = Vec::new();
    let mut remaining_area = grid_area;
    let mut resolver = PaneResolver::with_capacity(panes.len());
    let mut unassigned_container: Option<(Rect, Option<LayoutDirection>)> = None;

    for item in layout_items {
        let carved_area = if let Some(edge) = item.edge {
            let size = item.size.unwrap_or(100);
            let (slice, next) = carve_layout_edge(remaining_area, edge, size);
            remaining_area = next;
            slice
        } else {
            remaining_area
        };

        process_layout_node(
            item,
            carved_area,
            panes,
            &mut resolver,
            &mut geometries,
            &mut unassigned_container,
        );
    }

    let unassigned_panes: Vec<&ProcessPane> = panes
        .iter()
        .filter(|p| !resolver.assigned_panes.contains(&p.id))
        .collect();

    let (target_area, configured_direction) = unassigned_container.unwrap_or((remaining_area, None));
    layout_unassigned_panes(&unassigned_panes, target_area, configured_direction, &mut geometries);

    geometries
}

/// Recursively processes a layout node, assigning pane geometries or splitting
/// the area among children.
fn process_layout_node(
    node: &LayoutNode,
    area: Rect,
    panes: &[ProcessPane],
    resolver: &mut PaneResolver,
    geometries: &mut Vec<PaneGeometry>,
    unassigned_container: &mut Option<(Rect, Option<LayoutDirection>)>,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    if node.unassigned.unwrap_or_default() {
        unassigned_container.get_or_insert((area, node.direction));
        return;
    }

    if let Some(name) = &node.name {
        if let Some(target) = resolver.resolve_target(name, panes) {
            geometries.push(PaneGeometry::new(target, area));
        }
        return;
    }

    if let Some(children) = &node.items {
        if children.is_empty() {
            return;
        }

        let direction = match node.direction {
            Some(LayoutDirection::Horizontal) => Direction::Horizontal,
            Some(LayoutDirection::Vertical) => Direction::Vertical,
            None => match node.edge {
                Some(LayoutEdge::Left | LayoutEdge::Right) => Direction::Vertical,
                _ => Direction::Horizontal,
            },
        };

        let total_specified: u16 = children.iter().filter_map(|c| c.size).sum();
        let unspecified_count = children.iter().filter(|c| c.size.is_none()).count();
        let remaining_pct = 100_u16.saturating_sub(total_specified);
        let default_pct = if unspecified_count > 0 {
            remaining_pct / unspecified_count as u16
        } else {
            0
        };

        let constraints: Vec<Constraint> = children
            .iter()
            .map(|c| Constraint::Percentage(c.size.unwrap_or(default_pct).min(100)))
            .collect();

        let chunks = Layout::default()
            .direction(direction)
            .constraints(constraints)
            .split(area);

        for (child, &child_area) in children.iter().zip(chunks.iter()) {
            process_layout_node(child, child_area, panes, resolver, geometries, unassigned_container);
        }
    }
}

/// Carves a percentage-sized slice off the specified edge of an area,
/// returning `(carved_slice, remaining_area)`.
fn carve_layout_edge(area: Rect, edge: LayoutEdge, size: u16) -> (Rect, Rect) {
    let percentage = size.min(100);
    let (direction, constraints) = match edge {
        LayoutEdge::Left => (
            Direction::Horizontal,
            [Constraint::Percentage(percentage), Constraint::Min(0)],
        ),
        LayoutEdge::Top => (
            Direction::Vertical,
            [Constraint::Percentage(percentage), Constraint::Min(0)],
        ),
        LayoutEdge::Right => (
            Direction::Horizontal,
            [Constraint::Min(0), Constraint::Percentage(percentage)],
        ),
        LayoutEdge::Bottom => (
            Direction::Vertical,
            [Constraint::Min(0), Constraint::Percentage(percentage)],
        ),
    };

    let chunks = Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area);

    match edge {
        LayoutEdge::Left | LayoutEdge::Top => (chunks[0], chunks[1]),
        LayoutEdge::Right | LayoutEdge::Bottom => (chunks[1], chunks[0]),
    }
}

/// Distributes any remaining unassigned panes into the target area,
/// arranging them in a balanced row-major or column-major grid (max 4 per group).
fn layout_unassigned_panes(
    unassigned_panes: &[&ProcessPane],
    target_area: Rect,
    configured_direction: Option<LayoutDirection>,
    geometries: &mut Vec<PaneGeometry>,
) {
    if unassigned_panes.is_empty() || target_area.width == 0 || target_area.height == 0 {
        return;
    }

    let num_panes = unassigned_panes.len();
    let use_horizontal = match configured_direction {
        Some(LayoutDirection::Horizontal) => true,
        Some(LayoutDirection::Vertical) => false,
        None => target_area.width >= target_area.height * 2,
    };

    let max_per_group = 4;
    let num_groups = num_panes.div_ceil(max_per_group);

    let (primary_dir, secondary_dir) = if use_horizontal {
        (Direction::Vertical, Direction::Horizontal)
    } else {
        (Direction::Horizontal, Direction::Vertical)
    };

    let primary_areas = Layout::default()
        .direction(primary_dir)
        .constraints(vec![Constraint::Ratio(1, num_groups as u32); num_groups])
        .split(target_area);

    let base_size = num_panes / num_groups;
    let remainder = num_panes % num_groups;
    let mut start_idx = 0;

    for (group_idx, &group_area) in primary_areas.iter().enumerate() {
        let group_len = base_size + if group_idx < remainder { 1 } else { 0 };
        let end_idx = start_idx + group_len;
        let group_panes = &unassigned_panes[start_idx..end_idx];
        start_idx = end_idx;

        let secondary_areas = Layout::default()
            .direction(secondary_dir)
            .constraints(vec![Constraint::Ratio(1, group_len as u32); group_len])
            .split(group_area);

        for (pane, &area) in group_panes.iter().zip(secondary_areas.iter()) {
            geometries.push(PaneGeometry::new(PaneTarget::Process(pane.id), area));
        }
    }
}
