use std::collections::{BTreeMap, BTreeSet};

use shell_core::MonitorId;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AppBarMode {
    #[default]
    Preview,
    Shell,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MonitorGeometry {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
    pub work_bottom: i32,
    pub dpi: u32,
    pub primary: bool,
}

impl MonitorGeometry {
    pub fn valid(self) -> bool {
        self.right > self.left && self.bottom > self.top && self.dpi > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorBar {
    pub monitor_id: MonitorId,
    pub hwnd_identity: u64,
    pub geometry: MonitorGeometry,
    pub generation: u64,
    pub registered: bool,
    pub reserved_bottom: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppBarEffect {
    Register {
        monitor_id: MonitorId,
        hwnd_identity: u64,
    },
    QueryAndPosition {
        monitor_id: MonitorId,
        thickness: i32,
    },
    Remove {
        monitor_id: MonitorId,
        hwnd_identity: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurfaceChange {
    Created(MonitorId),
    Updated(MonitorId),
    Removed(MonitorId),
    PrimaryChanged(MonitorId),
}

#[derive(Debug, Default)]
pub struct AppBarRegistry {
    mode: AppBarMode,
    generation: u64,
    next_hwnd_identity: u64,
    bars: BTreeMap<MonitorId, MonitorBar>,
}

impl AppBarRegistry {
    pub fn new(mode: AppBarMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }
    pub fn bars(&self) -> &BTreeMap<MonitorId, MonitorBar> {
        &self.bars
    }

    pub fn reconcile(
        &mut self,
        monitors: impl IntoIterator<Item = (MonitorId, MonitorGeometry)>,
        thickness: i32,
    ) -> Result<(Vec<SurfaceChange>, Vec<AppBarEffect>), &'static str> {
        if thickness <= 0 {
            return Err("taskbar-thickness-invalid");
        }
        let incoming: BTreeMap<_, _> = monitors.into_iter().collect();
        if incoming.is_empty() || incoming.values().any(|geometry| !geometry.valid()) {
            return Err("taskbar-monitor-set-invalid");
        }
        self.generation = self.generation.saturating_add(1);
        let mut changes = Vec::new();
        let mut effects = Vec::new();
        let incoming_ids: BTreeSet<_> = incoming.keys().cloned().collect();
        let removed: Vec<_> = self
            .bars
            .keys()
            .filter(|id| !incoming_ids.contains(*id))
            .cloned()
            .collect();
        for id in removed {
            if let Some(bar) = self.bars.remove(&id) {
                if self.mode == AppBarMode::Shell && bar.registered {
                    effects.push(AppBarEffect::Remove {
                        monitor_id: id.clone(),
                        hwnd_identity: bar.hwnd_identity,
                    });
                }
                changes.push(SurfaceChange::Removed(id));
            }
        }
        for (id, geometry) in incoming {
            if let Some(bar) = self.bars.get_mut(&id) {
                if bar.geometry != geometry {
                    let primary_changed =
                        bar.geometry.primary != geometry.primary && geometry.primary;
                    bar.geometry = geometry;
                    bar.generation = self.generation;
                    bar.reserved_bottom = if self.mode == AppBarMode::Shell {
                        thickness
                    } else {
                        0
                    };
                    if self.mode == AppBarMode::Shell {
                        effects.push(AppBarEffect::QueryAndPosition {
                            monitor_id: id.clone(),
                            thickness,
                        });
                    }
                    changes.push(SurfaceChange::Updated(id.clone()));
                    if primary_changed {
                        changes.push(SurfaceChange::PrimaryChanged(id));
                    }
                }
            } else {
                self.next_hwnd_identity = self.next_hwnd_identity.saturating_add(1);
                let hwnd_identity = self.next_hwnd_identity;
                let registered = self.mode == AppBarMode::Shell;
                self.bars.insert(
                    id.clone(),
                    MonitorBar {
                        monitor_id: id.clone(),
                        hwnd_identity,
                        geometry,
                        generation: self.generation,
                        registered,
                        reserved_bottom: if registered { thickness } else { 0 },
                    },
                );
                if registered {
                    effects.push(AppBarEffect::Register {
                        monitor_id: id.clone(),
                        hwnd_identity,
                    });
                    effects.push(AppBarEffect::QueryAndPosition {
                        monitor_id: id.clone(),
                        thickness,
                    });
                }
                changes.push(SurfaceChange::Created(id));
            }
        }
        Ok((changes, effects))
    }

    pub fn teardown(&mut self) -> Vec<AppBarEffect> {
        self.bars
            .values_mut()
            .filter_map(|bar| {
                if bar.registered {
                    bar.registered = false;
                    bar.reserved_bottom = 0;
                    Some(AppBarEffect::Remove {
                        monitor_id: bar.monitor_id.clone(),
                        hwnd_identity: bar.hwnd_identity,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn registration_failed(&mut self, monitor_id: &MonitorId) -> bool {
        let Some(bar) = self.bars.get_mut(monitor_id) else {
            return false;
        };
        bar.registered = false;
        bar.reserved_bottom = 0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn monitor(name: &str, dpi: u32, primary: bool) -> (MonitorId, MonitorGeometry) {
        (
            MonitorId::new(name).unwrap(),
            MonitorGeometry {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1080,
                work_bottom: 1040,
                dpi,
                primary,
            },
        )
    }
    #[test]
    fn preview_maps_one_bar_per_monitor_without_work_area_effects() {
        let mut registry = AppBarRegistry::new(AppBarMode::Preview);
        let (_, effects) = registry
            .reconcile([monitor("a", 96, true), monitor("b", 144, false)], 80)
            .unwrap();
        assert!(effects.is_empty());
        assert_eq!(registry.bars().len(), 2);
        assert!(registry.bars().values().all(|bar| bar.reserved_bottom == 0));
    }
    #[test]
    fn shell_reconciles_dpi_remove_and_idempotent_teardown() {
        let mut registry = AppBarRegistry::new(AppBarMode::Shell);
        let (_, effects) = registry
            .reconcile([monitor("a", 96, true), monitor("b", 144, false)], 80)
            .unwrap();
        assert_eq!(effects.len(), 4);
        let (_, effects) = registry.reconcile([monitor("a", 192, true)], 96).unwrap();
        assert!(
            effects.iter().any(
                |e| matches!(e,AppBarEffect::Remove{monitor_id,..} if monitor_id.as_str()=="b")
            )
        );
        assert_eq!(registry.teardown().len(), 1);
        assert!(registry.teardown().is_empty());
    }
    #[test]
    fn registration_failure_returns_to_unreserved_state() {
        let mut registry = AppBarRegistry::new(AppBarMode::Shell);
        registry.reconcile([monitor("a", 96, true)], 80).unwrap();
        assert!(registry.registration_failed(&MonitorId::new("a").unwrap()));
        assert_eq!(registry.bars().values().next().unwrap().reserved_bottom, 0);
        assert!(registry.teardown().is_empty());
    }
}
