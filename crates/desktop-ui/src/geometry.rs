use std::collections::{BTreeMap, BTreeSet};

use shell_core::MonitorId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Dpi {
    pub x: u32,
    pub y: u32,
}

impl Dpi {
    pub const BASE: Self = Self { x: 96, y: 96 };

    pub fn new(x: u32, y: u32) -> Option<Self> {
        (x > 0 && y > 0).then_some(Self { x, y })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LogicalRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl LogicalRect {
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Option<Self> {
        (right > left && bottom > top).then_some(Self {
            left,
            top,
            right,
            bottom,
        })
    }

    pub fn width(self) -> f32 {
        self.right - self.left
    }
    pub fn height(self) -> f32 {
        self.bottom - self.top
    }
    pub fn intersects(self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
    pub fn clamp_point(
        self,
        point: LogicalPoint,
        item_width: f32,
        item_height: f32,
    ) -> LogicalPoint {
        LogicalPoint {
            x: point
                .x
                .clamp(self.left, (self.right - item_width).max(self.left)),
            y: point
                .y
                .clamp(self.top, (self.bottom - item_height).max(self.top)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonitorDescriptor {
    pub id: MonitorId,
    pub physical_bounds: (i32, i32, i32, i32),
    pub physical_work_area: (i32, i32, i32, i32),
    pub dpi: Dpi,
    pub primary: bool,
}

impl MonitorDescriptor {
    pub fn logical_bounds(&self) -> LogicalRect {
        scale_rect(self.physical_bounds, self.dpi)
    }

    pub fn logical_work_area(&self) -> LogicalRect {
        scale_rect(self.physical_work_area, self.dpi)
    }
}

fn scale_rect(rect: (i32, i32, i32, i32), dpi: Dpi) -> LogicalRect {
    let sx = 96.0 / dpi.x as f32;
    let sy = 96.0 / dpi.y as f32;
    LogicalRect {
        left: rect.0 as f32 * sx,
        top: rect.1 as f32 * sy,
        right: rect.2 as f32 * sx,
        bottom: rect.3 as f32 * sy,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SurfaceState {
    pub monitor: MonitorDescriptor,
    pub logical_bounds: LogicalRect,
    pub logical_work_area: LogicalRect,
    pub generation: u64,
    pub bottommost: bool,
    pub activates_on_show: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceChange {
    Created(MonitorId),
    Updated(MonitorId),
    Removed(MonitorId),
    PrimaryChanged(MonitorId),
}

#[derive(Clone, Debug, Default)]
pub struct DesktopSurfaceRegistry {
    surfaces: BTreeMap<MonitorId, SurfaceState>,
    generation: u64,
}

impl DesktopSurfaceRegistry {
    pub fn reconcile(
        &mut self,
        monitors: impl IntoIterator<Item = MonitorDescriptor>,
    ) -> Vec<SurfaceChange> {
        self.generation = self.generation.saturating_add(1);
        let monitors: BTreeMap<_, _> = monitors
            .into_iter()
            .map(|monitor| (monitor.id.clone(), monitor))
            .collect();
        let incoming: BTreeSet<_> = monitors.keys().cloned().collect();
        let existing: Vec<_> = self.surfaces.keys().cloned().collect();
        let mut changes = Vec::new();
        for id in existing {
            if !incoming.contains(&id) {
                self.surfaces.remove(&id);
                changes.push(SurfaceChange::Removed(id));
            }
        }
        for (id, monitor) in monitors {
            let logical_bounds = monitor.logical_bounds();
            let logical_work_area = monitor.logical_work_area();
            match self.surfaces.get_mut(&id) {
                None => {
                    self.surfaces.insert(
                        id.clone(),
                        SurfaceState {
                            monitor,
                            logical_bounds,
                            logical_work_area,
                            generation: self.generation,
                            bottommost: true,
                            activates_on_show: false,
                        },
                    );
                    changes.push(SurfaceChange::Created(id));
                }
                Some(surface) => {
                    let primary_changed =
                        surface.monitor.primary != monitor.primary && monitor.primary;
                    if surface.monitor != monitor {
                        surface.monitor = monitor;
                        surface.logical_bounds = logical_bounds;
                        surface.logical_work_area = logical_work_area;
                        surface.generation = self.generation;
                        changes.push(SurfaceChange::Updated(id.clone()));
                    }
                    if primary_changed {
                        changes.push(SurfaceChange::PrimaryChanged(id));
                    }
                }
            }
        }
        changes
    }

    pub fn surfaces(&self) -> &BTreeMap<MonitorId, SurfaceState> {
        &self.surfaces
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor(id: &str, dpi: u32, primary: bool, left: i32) -> MonitorDescriptor {
        MonitorDescriptor {
            id: MonitorId::new(id).unwrap(),
            physical_bounds: (left, 0, left + 1920, 1080),
            physical_work_area: (left, 0, left + 1920, 1000),
            dpi: Dpi::new(dpi, dpi).unwrap(),
            primary,
        }
    }

    #[test]
    fn one_surface_per_monitor_and_no_orphans_after_hotplug() {
        let mut registry = DesktopSurfaceRegistry::default();
        assert_eq!(
            registry
                .reconcile([monitor("a", 96, true, 0), monitor("b", 144, false, 1920)])
                .len(),
            2
        );
        assert_eq!(registry.surfaces().len(), 2);
        let changes = registry.reconcile([monitor("b", 192, true, 0)]);
        assert!(changes.contains(&SurfaceChange::Removed(MonitorId::new("a").unwrap())));
        assert_eq!(registry.surfaces().len(), 1);
        assert!(
            registry
                .surfaces()
                .values()
                .all(|surface| surface.bottommost && !surface.activates_on_show)
        );
    }

    #[test]
    fn dpi_and_work_area_use_logical_units() {
        let monitor = monitor("scaled", 192, true, 0);
        assert_eq!(monitor.logical_bounds().width(), 960.0);
        assert_eq!(monitor.logical_work_area().height(), 500.0);
    }
}
