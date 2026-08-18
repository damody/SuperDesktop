pub const AUTO_HIDE_DELAY_MS: u64 = 500;
pub const AUTO_HIDE_REVEAL_PIXELS: i32 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalPoint {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl PhysicalRect {
    pub fn contains(self, point: PhysicalPoint) -> bool {
        point.x >= self.left && point.x < self.right && point.y >= self.top && point.y < self.bottom
    }

    pub const fn width(self) -> i32 {
        self.right - self.left
    }

    pub const fn height(self) -> i32 {
        self.bottom - self.top
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutoHideEndpoints {
    pub visible: PhysicalRect,
    pub hidden: PhysicalRect,
    pub reveal: PhysicalRect,
}

pub fn auto_hide_endpoints(
    left: i32,
    right: i32,
    anchor_bottom: i32,
    visible_height: i32,
) -> Option<AutoHideEndpoints> {
    if right <= left || visible_height <= AUTO_HIDE_REVEAL_PIXELS {
        return None;
    }
    let visible = PhysicalRect {
        left,
        top: anchor_bottom.saturating_sub(visible_height),
        right,
        bottom: anchor_bottom,
    };
    let hidden_top = anchor_bottom.saturating_sub(AUTO_HIDE_REVEAL_PIXELS);
    Some(AutoHideEndpoints {
        visible,
        hidden: PhysicalRect {
            left,
            top: hidden_top,
            right,
            bottom: hidden_top.saturating_add(visible_height),
        },
        reveal: PhysicalRect {
            left,
            top: hidden_top,
            right,
            bottom: anchor_bottom,
        },
    })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AutoHideState {
    #[default]
    Visible,
    HidePending {
        deadline_ms: u64,
    },
    Hidden,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AutoHideEffect {
    #[default]
    NoChange,
    Show,
    Hide,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AutoHideInput {
    pub enabled: bool,
    pub now_ms: u64,
    pub pointer: Option<PhysicalPoint>,
    pub visibility_hold: bool,
    pub endpoints: AutoHideEndpoints,
}

pub fn reduce_auto_hide(
    state: AutoHideState,
    input: AutoHideInput,
) -> (AutoHideState, AutoHideEffect) {
    if !input.enabled {
        return match state {
            AutoHideState::Hidden => (AutoHideState::Visible, AutoHideEffect::Show),
            AutoHideState::HidePending { .. } => (AutoHideState::Visible, AutoHideEffect::NoChange),
            AutoHideState::Visible => (state, AutoHideEffect::NoChange),
        };
    }
    let Some(pointer) = input.pointer else {
        return (state, AutoHideEffect::NoChange);
    };
    if input.visibility_hold {
        return match state {
            AutoHideState::Hidden => (AutoHideState::Visible, AutoHideEffect::Show),
            _ => (AutoHideState::Visible, AutoHideEffect::NoChange),
        };
    }
    match state {
        AutoHideState::Hidden => {
            if input.endpoints.reveal.contains(pointer) {
                (AutoHideState::Visible, AutoHideEffect::Show)
            } else {
                (state, AutoHideEffect::NoChange)
            }
        }
        AutoHideState::Visible => {
            if input.endpoints.visible.contains(pointer) {
                (state, AutoHideEffect::NoChange)
            } else {
                (
                    AutoHideState::HidePending {
                        deadline_ms: input.now_ms.saturating_add(AUTO_HIDE_DELAY_MS),
                    },
                    AutoHideEffect::NoChange,
                )
            }
        }
        AutoHideState::HidePending { deadline_ms } => {
            if input.endpoints.visible.contains(pointer) {
                (AutoHideState::Visible, AutoHideEffect::NoChange)
            } else if input.now_ms >= deadline_ms {
                (AutoHideState::Hidden, AutoHideEffect::Hide)
            } else {
                (state, AutoHideEffect::NoChange)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoints() -> AutoHideEndpoints {
        auto_hide_endpoints(-1920, 0, 1080, 120).unwrap()
    }

    fn input(now_ms: u64, pointer: Option<PhysicalPoint>) -> AutoHideInput {
        AutoHideInput {
            enabled: true,
            now_ms,
            pointer,
            visibility_hold: false,
            endpoints: endpoints(),
        }
    }

    #[test]
    fn endpoint_geometry_preserves_negative_origin_height_and_two_pixel_edge() {
        let value = endpoints();
        assert_eq!(
            value.visible,
            PhysicalRect {
                left: -1920,
                top: 960,
                right: 0,
                bottom: 1080
            }
        );
        assert_eq!(value.visible.height(), 120);
        assert_eq!(value.hidden.top, 1078);
        assert_eq!(value.hidden.height(), 120);
        assert_eq!(value.reveal.height(), 2);
        assert_eq!(value.reveal.width(), 1920);
        assert!(auto_hide_endpoints(0, 0, 100, 70).is_none());
        assert!(auto_hide_endpoints(0, 100, 100, 2).is_none());
    }

    #[test]
    fn hide_requires_full_delay_and_reveal_is_immediate() {
        let outside = Some(PhysicalPoint { x: -100, y: 100 });
        let (pending, effect) = reduce_auto_hide(AutoHideState::Visible, input(10, outside));
        assert_eq!(pending, AutoHideState::HidePending { deadline_ms: 510 });
        assert_eq!(effect, AutoHideEffect::NoChange);
        assert_eq!(
            reduce_auto_hide(pending, input(509, outside)),
            (pending, AutoHideEffect::NoChange)
        );
        assert_eq!(
            reduce_auto_hide(pending, input(510, outside)),
            (AutoHideState::Hidden, AutoHideEffect::Hide)
        );
        let edge = Some(PhysicalPoint { x: -1, y: 1079 });
        assert_eq!(
            reduce_auto_hide(AutoHideState::Hidden, input(511, edge)),
            (AutoHideState::Visible, AutoHideEffect::Show)
        );
    }

    #[test]
    fn pointer_return_hold_disable_and_cursor_failure_are_idempotent() {
        let outside = Some(PhysicalPoint { x: -100, y: 100 });
        let pending = reduce_auto_hide(AutoHideState::Visible, input(0, outside)).0;
        let inside = Some(PhysicalPoint { x: -100, y: 1000 });
        assert_eq!(
            reduce_auto_hide(pending, input(499, inside)),
            (AutoHideState::Visible, AutoHideEffect::NoChange)
        );
        let mut held = input(600, outside);
        held.visibility_hold = true;
        assert_eq!(
            reduce_auto_hide(AutoHideState::Hidden, held),
            (AutoHideState::Visible, AutoHideEffect::Show)
        );
        let mut disabled = held;
        disabled.enabled = false;
        assert_eq!(
            reduce_auto_hide(AutoHideState::Hidden, disabled),
            (AutoHideState::Visible, AutoHideEffect::Show)
        );
        assert_eq!(
            reduce_auto_hide(pending, input(10_000, None)),
            (pending, AutoHideEffect::NoChange)
        );
    }
}
