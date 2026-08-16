use std::collections::{BTreeMap, BTreeSet};

use shell_core::{MonitorId, ShellItemId};

use crate::{LogicalPoint, LogicalRect};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DesktopSortKey {
    Name,
    Kind,
    Size,
    Modified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DesktopSortRecord {
    pub identity: ShellItemId,
    pub name: String,
    pub kind: String,
    pub size: u64,
    pub modified_unix_ms: u64,
}

pub fn sort_desktop_records(
    records: &mut [DesktopSortRecord],
    key: DesktopSortKey,
    direction: SortDirection,
) {
    records.sort_by(|left, right| {
        let primary = match key {
            DesktopSortKey::Name => natural_cmp(&left.name, &right.name),
            DesktopSortKey::Kind => left.kind.to_lowercase().cmp(&right.kind.to_lowercase()),
            DesktopSortKey::Size => left.size.cmp(&right.size),
            DesktopSortKey::Modified => left.modified_unix_ms.cmp(&right.modified_unix_ms),
        };
        let ordered = if direction == SortDirection::Descending {
            primary.reverse()
        } else {
            primary
        };
        ordered.then_with(|| left.identity.cmp(&right.identity))
    });
}

fn natural_cmp(left: &str, right: &str) -> std::cmp::Ordering {
    let left = natural_parts(left);
    let right = natural_parts(right);
    left.cmp(&right)
}

fn natural_parts(value: &str) -> Vec<(bool, u64, String)> {
    let mut output = Vec::new();
    let mut current = String::new();
    let mut digits = None;
    for character in value.to_lowercase().chars() {
        let is_digit = character.is_ascii_digit();
        if digits.is_some_and(|state| state != is_digit) {
            let number = current.parse().unwrap_or(0);
            output.push((
                digits.unwrap_or(false),
                number,
                std::mem::take(&mut current),
            ));
        }
        digits = Some(is_digit);
        current.push(character);
    }
    if !current.is_empty() {
        let number = current.parse().unwrap_or(0);
        output.push((digits.unwrap_or(false), number, current));
    }
    output
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridMetrics {
    pub cell_width: f32,
    pub cell_height: f32,
    pub icon_size: f32,
    pub label_height: f32,
}

impl GridMetrics {
    pub const WINDOWS_10: Self = Self {
        cell_width: 80.0,
        cell_height: 92.0,
        icon_size: 48.0,
        label_height: 32.0,
    };
    pub fn physical(self, dpi: u32) -> Self {
        let scale = dpi as f32 / 96.0;
        Self {
            cell_width: self.cell_width * scale,
            cell_height: self.cell_height * scale,
            icon_size: self.icon_size * scale,
            label_height: self.label_height * scale,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PersistedPosition {
    pub monitor_id: MonitorId,
    pub item_id: ShellItemId,
    pub logical: LogicalPoint,
    pub layout_revision: u64,
}

#[derive(Clone, Debug, Default)]
pub struct DesktopLayout {
    positions: BTreeMap<ShellItemId, PersistedPosition>,
    revision: u64,
}

impl DesktopLayout {
    pub fn restore_snapshot(
        &mut self,
        snapshot: impl IntoIterator<Item = PersistedPosition>,
        available: &BTreeSet<ShellItemId>,
        work_areas: &BTreeMap<MonitorId, LogicalRect>,
        fallback_monitor: &MonitorId,
        metrics: GridMetrics,
    ) {
        self.revision = self.revision.saturating_add(1);
        self.positions.clear();
        let fallback_area = work_areas.get(fallback_monitor).copied();
        for mut position in snapshot {
            if !available.contains(&position.item_id) {
                continue;
            }
            let (monitor, area) = work_areas
                .get(&position.monitor_id)
                .map(|area| (position.monitor_id.clone(), *area))
                .or_else(|| fallback_area.map(|area| (fallback_monitor.clone(), area)))
                .unwrap_or((
                    position.monitor_id.clone(),
                    LogicalRect::new(0.0, 0.0, metrics.cell_width, metrics.cell_height)
                        .expect("valid fallback"),
                ));
            position.monitor_id = monitor;
            position.logical =
                area.clamp_point(position.logical, metrics.cell_width, metrics.cell_height);
            position.layout_revision = self.revision;
            self.positions.insert(position.item_id.clone(), position);
        }
        if let Some(area) = fallback_area {
            self.resolve_collisions(area, metrics);
        }
    }

    pub fn align_to_grid(&mut self, work_area: LogicalRect, metrics: GridMetrics) {
        self.revision = self.revision.saturating_add(1);
        for position in self.positions.values_mut() {
            let x = ((position.logical.x - work_area.left) / metrics.cell_width).round();
            let y = ((position.logical.y - work_area.top) / metrics.cell_height).round();
            position.logical = work_area.clamp_point(
                LogicalPoint {
                    x: work_area.left + x * metrics.cell_width,
                    y: work_area.top + y * metrics.cell_height,
                },
                metrics.cell_width,
                metrics.cell_height,
            );
            position.layout_revision = self.revision;
        }
        self.resolve_collisions(work_area, metrics);
    }
    pub fn arrange(
        &mut self,
        monitor_id: &MonitorId,
        items: &[ShellItemId],
        work_area: LogicalRect,
        metrics: GridMetrics,
    ) {
        self.revision = self.revision.saturating_add(1);
        let rows = ((work_area.height() / metrics.cell_height).floor() as usize).max(1);
        for (index, item) in items.iter().enumerate() {
            self.positions.entry(item.clone()).or_insert_with(|| {
                let column = index / rows;
                let row = index % rows;
                PersistedPosition {
                    monitor_id: monitor_id.clone(),
                    item_id: item.clone(),
                    logical: LogicalPoint {
                        x: work_area.left + column as f32 * metrics.cell_width,
                        y: work_area.top + row as f32 * metrics.cell_height,
                    },
                    layout_revision: self.revision,
                }
            });
        }
        self.positions.retain(|item, _| items.contains(item));
        for position in self.positions.values_mut() {
            position.logical =
                work_area.clamp_point(position.logical, metrics.cell_width, metrics.cell_height);
            position.layout_revision = self.revision;
        }
        self.resolve_collisions(work_area, metrics);
    }

    pub fn reposition(
        &mut self,
        item: &ShellItemId,
        point: LogicalPoint,
        work_area: LogicalRect,
        metrics: GridMetrics,
    ) -> bool {
        let Some(position) = self.positions.get_mut(item) else {
            return false;
        };
        self.revision = self.revision.saturating_add(1);
        position.logical = work_area.clamp_point(point, metrics.cell_width, metrics.cell_height);
        position.layout_revision = self.revision;
        self.resolve_collisions(work_area, metrics);
        true
    }

    pub fn remap_monitor(
        &mut self,
        missing: &MonitorId,
        replacement: &MonitorId,
        work_area: LogicalRect,
        metrics: GridMetrics,
    ) {
        self.revision = self.revision.saturating_add(1);
        for position in self
            .positions
            .values_mut()
            .filter(|position| &position.monitor_id == missing)
        {
            position.monitor_id = replacement.clone();
            position.logical =
                work_area.clamp_point(position.logical, metrics.cell_width, metrics.cell_height);
            position.layout_revision = self.revision;
        }
        self.resolve_collisions(work_area, metrics);
    }
    pub fn positions(&self) -> &BTreeMap<ShellItemId, PersistedPosition> {
        &self.positions
    }

    fn resolve_collisions(&mut self, work_area: LogicalRect, metrics: GridMetrics) {
        let mut occupied = BTreeSet::new();
        for position in self.positions.values_mut() {
            let mut column = ((position.logical.x - work_area.left) / metrics.cell_width)
                .round()
                .max(0.0) as i32;
            let mut row = ((position.logical.y - work_area.top) / metrics.cell_height)
                .round()
                .max(0.0) as i32;
            let max_rows = ((work_area.height() / metrics.cell_height).floor() as i32).max(1);
            while !occupied.insert((column, row)) {
                row += 1;
                if row >= max_rows {
                    row = 0;
                    column += 1;
                }
            }
            position.logical = work_area.clamp_point(
                LogicalPoint {
                    x: work_area.left + column as f32 * metrics.cell_width,
                    y: work_area.top + row as f32 * metrics.cell_height,
                },
                metrics.cell_width,
                metrics.cell_height,
            );
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionGesture {
    Single(ShellItemId),
    CtrlToggle(ShellItemId),
    ShiftRange(ShellItemId),
    RubberBand(BTreeSet<ShellItemId>),
    Clear,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionModel {
    pub selected: BTreeSet<ShellItemId>,
    pub focused: Option<ShellItemId>,
    anchor: Option<ShellItemId>,
}

impl SelectionModel {
    pub fn apply(&mut self, gesture: SelectionGesture, order: &[ShellItemId]) {
        match gesture {
            SelectionGesture::Single(item) => {
                self.selected = [item.clone()].into_iter().collect();
                self.focused = Some(item.clone());
                self.anchor = Some(item);
            }
            SelectionGesture::CtrlToggle(item) => {
                if !self.selected.remove(&item) {
                    self.selected.insert(item.clone());
                }
                self.focused = Some(item.clone());
                self.anchor = Some(item);
            }
            SelectionGesture::ShiftRange(item) => {
                let anchor = self
                    .anchor
                    .as_ref()
                    .or(self.focused.as_ref())
                    .unwrap_or(&item);
                if let (Some(a), Some(b)) = (
                    order.iter().position(|value| value == anchor),
                    order.iter().position(|value| value == &item),
                ) {
                    let (start, end) = if a <= b { (a, b) } else { (b, a) };
                    self.selected = order[start..=end].iter().cloned().collect();
                }
                self.focused = Some(item);
            }
            SelectionGesture::RubberBand(items) => {
                self.selected = items;
                self.focused = self.selected.iter().next().cloned();
            }
            SelectionGesture::Clear => {
                self.selected.clear();
                self.focused = None;
                self.anchor = None;
            }
        }
    }
    pub fn restore(&mut self, available: &BTreeSet<ShellItemId>) {
        self.selected.retain(|item| available.contains(item));
        if self
            .focused
            .as_ref()
            .is_some_and(|item| !available.contains(item))
        {
            self.focused = None;
        }
        if self
            .anchor
            .as_ref()
            .is_some_and(|item| !available.contains(item))
        {
            self.anchor = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(value: &str) -> ShellItemId {
        ShellItemId::new(value).unwrap()
    }
    fn area() -> LogicalRect {
        LogicalRect::new(0.0, 0.0, 240.0, 276.0).unwrap()
    }
    #[test]
    fn grid_selection_and_collision_are_deterministic() {
        let items = vec![id("a"), id("b"), id("c"), id("d")];
        let monitor = MonitorId::new("m").unwrap();
        let mut left = DesktopLayout::default();
        let mut right = DesktopLayout::default();
        left.arrange(&monitor, &items, area(), GridMetrics::WINDOWS_10);
        right.arrange(&monitor, &items, area(), GridMetrics::WINDOWS_10);
        assert_eq!(left.positions(), right.positions());
        left.reposition(
            &items[1],
            LogicalPoint { x: 0.0, y: 0.0 },
            area(),
            GridMetrics::WINDOWS_10,
        );
        let unique: BTreeSet<_> = left
            .positions()
            .values()
            .map(|p| (p.logical.x as i32, p.logical.y as i32))
            .collect();
        assert_eq!(unique.len(), items.len());
    }
    #[test]
    fn ctrl_shift_and_rubber_band_match_expected_items() {
        let order = vec![id("a"), id("b"), id("c")];
        let mut selection = SelectionModel::default();
        selection.apply(SelectionGesture::Single(order[0].clone()), &order);
        selection.apply(SelectionGesture::ShiftRange(order[2].clone()), &order);
        assert_eq!(selection.selected.len(), 3);
        selection.apply(SelectionGesture::CtrlToggle(order[1].clone()), &order);
        assert!(!selection.selected.contains(&order[1]));
        selection.apply(
            SelectionGesture::RubberBand([order[1].clone()].into_iter().collect()),
            &order,
        );
        assert_eq!(selection.selected, [order[1].clone()].into_iter().collect());
    }
    #[test]
    fn dpi_and_monitor_remap_keep_logical_positions_visible() {
        let item = id("a");
        let monitor = MonitorId::new("old").unwrap();
        let new = MonitorId::new("new").unwrap();
        let mut layout = DesktopLayout::default();
        layout.arrange(
            &monitor,
            std::slice::from_ref(&item),
            area(),
            GridMetrics::WINDOWS_10,
        );
        layout.reposition(
            &item,
            LogicalPoint { x: 999.0, y: 999.0 },
            area(),
            GridMetrics::WINDOWS_10,
        );
        layout.remap_monitor(&monitor, &new, area(), GridMetrics::WINDOWS_10);
        let position = &layout.positions()[&item];
        assert_eq!(position.monitor_id, new);
        assert!(position.logical.x <= 160.0 && position.logical.y <= 184.0);
    }

    #[test]
    fn natural_sort_and_snapshot_restore_are_stable() {
        let mut records = vec![
            DesktopSortRecord {
                identity: id("10"),
                name: "File 10".into(),
                kind: "txt".into(),
                size: 10,
                modified_unix_ms: 2,
            },
            DesktopSortRecord {
                identity: id("2"),
                name: "file 2".into(),
                kind: "txt".into(),
                size: 2,
                modified_unix_ms: 1,
            },
        ];
        sort_desktop_records(&mut records, DesktopSortKey::Name, SortDirection::Ascending);
        assert_eq!(records[0].identity, id("2"));

        let old = MonitorId::new("old").unwrap();
        let fallback = MonitorId::new("fallback").unwrap();
        let keep = id("keep");
        let snapshot = vec![PersistedPosition {
            monitor_id: old,
            item_id: keep.clone(),
            logical: LogicalPoint { x: 999.0, y: 999.0 },
            layout_revision: 1,
        }];
        let mut layout = DesktopLayout::default();
        layout.restore_snapshot(
            snapshot,
            &[keep.clone()].into_iter().collect(),
            &[(fallback.clone(), area())].into_iter().collect(),
            &fallback,
            GridMetrics::WINDOWS_10,
        );
        assert_eq!(layout.positions()[&keep].monitor_id, fallback);
        assert!(layout.positions()[&keep].logical.x <= 160.0);
    }
}
