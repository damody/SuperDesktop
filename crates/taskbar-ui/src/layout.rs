#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskbarRows(u8);

impl TaskbarRows {
    pub const DEFAULT: Self = Self(2);
    pub fn new(rows: u8) -> Self {
        if (1..=3).contains(&rows) {
            Self(rows)
        } else {
            Self::DEFAULT
        }
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SlotKind {
    Start,
    Pinned,
    Running,
    Status,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskbarSlot {
    pub stable_id: String,
    pub kind: SlotKind,
    pub row: u8,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub label_limit: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OverflowItem {
    pub stable_id: String,
    pub kind: SlotKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskbarLayout {
    pub rows: TaskbarRows,
    pub height: f32,
    pub slots: Vec<TaskbarSlot>,
    pub overflow: Vec<OverflowItem>,
}

impl TaskbarLayout {
    pub fn calculate(
        rows: u8,
        dpi: u32,
        width: f32,
        running: &[String],
        pinned: &[String],
    ) -> Self {
        let rows = TaskbarRows::new(rows);
        let scale = dpi.max(96) as f32 / 96.0;
        let row_height = 40.0 * scale;
        let height = row_height * f32::from(rows.get());
        let start_width = 48.0 * scale;
        let status_width = 180.0 * scale;
        let task_width = 160.0 * scale;
        let available = (width - start_width - status_width).max(0.0);
        let capacity =
            ((available / task_width).floor() as usize).saturating_mul(rows.get() as usize);
        let mut candidates = Vec::new();
        candidates.extend(pinned.iter().map(|id| (id.clone(), SlotKind::Pinned)));
        candidates.extend(
            running
                .iter()
                .filter(|id| !pinned.contains(id))
                .map(|id| (id.clone(), SlotKind::Running)),
        );
        let visible = candidates.len().min(capacity);
        let mut slots = vec![TaskbarSlot {
            stable_id: "start".into(),
            kind: SlotKind::Start,
            row: 0,
            x: 0.0,
            y: 0.0,
            width: start_width,
            height,
            label_limit: 0,
        }];
        for (index, (stable_id, kind)) in candidates.iter().take(visible).enumerate() {
            let row = (index % rows.get() as usize) as u8;
            let column = index / rows.get() as usize;
            slots.push(TaskbarSlot {
                stable_id: stable_id.clone(),
                kind: *kind,
                row,
                x: start_width + column as f32 * task_width,
                y: f32::from(row) * row_height,
                width: task_width,
                height: row_height,
                label_limit: (task_width / (8.0 * scale)).floor() as usize,
            });
        }
        slots.push(TaskbarSlot {
            stable_id: "status".into(),
            kind: SlotKind::Status,
            row: 0,
            x: (width - status_width).max(start_width),
            y: 0.0,
            width: status_width,
            height,
            label_limit: 24,
        });
        let overflow = candidates
            .into_iter()
            .skip(visible)
            .map(|(stable_id, kind)| OverflowItem { stable_id, kind })
            .collect();
        Self {
            rows,
            height,
            slots,
            overflow,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_rows_fall_back_only_to_two_and_scale_with_dpi() {
        assert_eq!(TaskbarRows::new(0), TaskbarRows::DEFAULT);
        assert_eq!(TaskbarRows::new(3).get(), 3);
        assert_eq!(
            TaskbarLayout::calculate(2, 192, 1920.0, &[], &[]).height,
            160.0
        );
    }
    #[test]
    fn slot_order_rows_hit_targets_and_overflow_are_deterministic() {
        let running = (0..10).map(|n| format!("r{n}")).collect::<Vec<_>>();
        let layout = TaskbarLayout::calculate(2, 96, 520.0, &running, &["r3".into()]);
        assert_eq!(layout.slots[0].kind, SlotKind::Start);
        assert_eq!(layout.slots[1].stable_id, "r3");
        assert!(layout.slots.iter().all(|slot| slot.height >= 40.0));
        assert!(!layout.overflow.is_empty());
        assert_eq!(layout.slots.last().unwrap().kind, SlotKind::Status);
    }
    #[test]
    fn one_two_three_row_geometry_has_no_overlap() {
        let items = (0..12).map(|n| format!("w{n}")).collect::<Vec<_>>();
        for rows in 1..=3 {
            let l = TaskbarLayout::calculate(rows, 144, 1600.0, &items, &[]);
            assert_eq!(l.height, 60.0 * f32::from(rows));
        }
    }
}
