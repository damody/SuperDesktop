use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::TestCase;

pub const GUI_MEASUREMENT_SCHEMA: &str = "gui-parity-measurement/v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestExplorerPolicy {
    RequiredAbsent,
    RecoveryOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuiArtifactKind {
    Measurement,
    Screenshot,
    Trace,
    TestLog,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuiArtifactSpec {
    pub name: &'static str,
    pub kind: GuiArtifactKind,
    pub required: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GuiVariant {
    pub mode: &'static str,
    pub theme: &'static str,
    pub locale: &'static str,
    pub dpi: u32,
    pub rows: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeometryRule {
    Width {
        region: &'static str,
        min: f64,
        max: f64,
    },
    Height {
        region: &'static str,
        min: f64,
        max: f64,
    },
    MinTarget {
        region: &'static str,
        width: f64,
        height: f64,
    },
    Contained {
        inner: &'static str,
        outer: &'static str,
    },
    NonOverlap {
        left: &'static str,
        right: &'static str,
    },
    Ratio {
        numerator: &'static str,
        denominator: &'static str,
        min: f64,
        max: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuiSurfaceSpec {
    pub id: &'static str,
    pub owner: &'static str,
    pub reference_family: &'static str,
    pub case_ids: &'static [&'static str],
    pub variants: &'static [GuiVariant],
    pub rules: &'static [GeometryRule],
    pub required_controls: &'static [&'static str],
    pub required_actions: &'static [&'static str],
    pub artifacts: &'static [GuiArtifactSpec],
    pub explorer_policy: ManifestExplorerPolicy,
    pub mandatory: bool,
}

const HEADFUL_ARTIFACTS: &[GuiArtifactSpec] = &[
    GuiArtifactSpec {
        name: "measurement",
        kind: GuiArtifactKind::Measurement,
        required: true,
    },
    GuiArtifactSpec {
        name: "screenshot",
        kind: GuiArtifactKind::Screenshot,
        required: true,
    },
    GuiArtifactSpec {
        name: "trace",
        kind: GuiArtifactKind::Trace,
        required: true,
    },
];
const UNIT_ARTIFACTS: &[GuiArtifactSpec] = &[GuiArtifactSpec {
    name: "test-log",
    kind: GuiArtifactKind::TestLog,
    required: true,
}];
const SCREEN_SNIP_ARTIFACTS: &[GuiArtifactSpec] = &[
    GuiArtifactSpec {
        name: "measurement",
        kind: GuiArtifactKind::Measurement,
        required: true,
    },
    GuiArtifactSpec {
        name: "trace",
        kind: GuiArtifactKind::Trace,
        required: true,
    },
];

const DEFAULT_VARIANTS: &[GuiVariant] = &[
    GuiVariant {
        mode: "preview",
        theme: "light",
        locale: "en-US",
        dpi: 96,
        rows: 1,
    },
    GuiVariant {
        mode: "shell",
        theme: "dark",
        locale: "zh-TW",
        dpi: 168,
        rows: 3,
    },
    GuiVariant {
        mode: "shell",
        theme: "high-contrast",
        locale: "en-US",
        dpi: 192,
        rows: 2,
    },
];

const TASKBAR_RULES: &[GeometryRule] = &[
    GeometryRule::Height {
        region: "window",
        min: 40.0,
        max: 120.0,
    },
    GeometryRule::MinTarget {
        region: "primary-control",
        width: 44.0,
        height: 40.0,
    },
    GeometryRule::MinTarget {
        region: "status-control",
        width: 36.0,
        height: 36.0,
    },
];
const START_RULES: &[GeometryRule] = &[
    GeometryRule::Width {
        region: "window",
        min: 624.0,
        max: 656.0,
    },
    GeometryRule::Height {
        region: "window",
        min: 1.0,
        max: 720.0,
    },
    GeometryRule::Contained {
        inner: "window",
        outer: "monitor",
    },
];
const SYSTEM_RULES: &[GeometryRule] = &[
    GeometryRule::Width {
        region: "window",
        min: 344.0,
        max: 396.0,
    },
    GeometryRule::Contained {
        inner: "window",
        outer: "monitor",
    },
    GeometryRule::Height {
        region: "taskbar-gap",
        min: 2.0,
        max: 16.0,
    },
];
const OVERFLOW_RULES: &[GeometryRule] = &[
    GeometryRule::Width {
        region: "window",
        min: 328.0,
        max: 360.0,
    },
    GeometryRule::MinTarget {
        region: "icon-cell",
        width: 40.0,
        height: 40.0,
    },
    GeometryRule::Contained {
        inner: "window",
        outer: "monitor",
    },
];
const CONTEXT_RULES: &[GeometryRule] = &[
    GeometryRule::Width {
        region: "window",
        min: 220.0,
        max: 240.0,
    },
    GeometryRule::MinTarget {
        region: "command-row",
        width: 1.0,
        height: 44.0,
    },
];
const POPUP_RULES: &[GeometryRule] = &[
    GeometryRule::Contained {
        inner: "window",
        outer: "monitor",
    },
    GeometryRule::Height {
        region: "taskbar-gap",
        min: 2.0,
        max: 16.0,
    },
];

pub fn gui_parity_manifest() -> Vec<GuiSurfaceSpec> {
    vec![
        spec(
            "taskbar",
            "taskbar-ui",
            &[
                "gui-taskbar-live",
                "gui-taskbar-resize",
                "gui-taskbar-auto-hide",
            ],
            TASKBAR_RULES,
        ),
        spec("start", "taskbar-ui", &["gui-start"], START_RULES),
        spec(
            "system-input",
            "taskbar-ui",
            &["gui-system-status"],
            SYSTEM_RULES,
        ),
        spec(
            "system-volume",
            "taskbar-ui",
            &["gui-system-status"],
            SYSTEM_RULES,
        ),
        spec(
            "system-network",
            "taskbar-ui",
            &["gui-system-status"],
            SYSTEM_RULES,
        ),
        spec(
            "calendar-notifications",
            "taskbar-ui",
            &["gui-system-status", "gui-notification-center"],
            SYSTEM_RULES,
        ),
        spec(
            "notification-overflow",
            "taskbar-ui",
            &["gui-notification-overflow"],
            OVERFLOW_RULES,
        ),
        spec(
            "taskbar-context",
            "taskbar-ui",
            &["gui-taskbar-live"],
            CONTEXT_RULES,
        ),
        spec(
            "system-context",
            "taskbar-ui",
            &["gui-system-status"],
            CONTEXT_RULES,
        ),
        spec(
            "jump-list",
            "taskbar-ui",
            &["gui-taskbar-window-actions"],
            POPUP_RULES,
        ),
        spec(
            "hover-preview",
            "taskbar-ui",
            &["gui-taskbar-hover-preview"],
            POPUP_RULES,
        ),
        GuiSurfaceSpec {
            id: "screen-snipping-shortcut",
            owner: "platform-win",
            reference_family: "windows-11-native-hotkey",
            case_ids: &["gui-win-shift-s-snipping"],
            variants: &[GuiVariant {
                mode: "shell",
                theme: "system",
                locale: "system",
                dpi: 96,
                rows: 1,
            }],
            rules: &[GeometryRule::Contained {
                inner: "snip-overlay",
                outer: "monitor",
            }],
            required_controls: &["screen-clipping-overlay"],
            required_actions: &["keyboard"],
            artifacts: SCREEN_SNIP_ARTIFACTS,
            explorer_policy: ManifestExplorerPolicy::RecoveryOnly,
            mandatory: true,
        },
        GuiSurfaceSpec {
            id: "task-view-alt-tab",
            owner: "taskbar-ui",
            reference_family: "windows-11-26200",
            case_ids: &["unit-taskbar-ui"],
            variants: DEFAULT_VARIANTS,
            rules: POPUP_RULES,
            required_controls: &["window"],
            required_actions: &["keyboard", "pointer"],
            artifacts: UNIT_ARTIFACTS,
            explorer_policy: ManifestExplorerPolicy::RequiredAbsent,
            mandatory: true,
        },
    ]
}

fn spec(
    id: &'static str,
    owner: &'static str,
    case_ids: &'static [&'static str],
    rules: &'static [GeometryRule],
) -> GuiSurfaceSpec {
    GuiSurfaceSpec {
        id,
        owner,
        reference_family: "windows-11-26200",
        case_ids,
        variants: DEFAULT_VARIANTS,
        rules,
        required_controls: &["window"],
        required_actions: &["pointer", "keyboard"],
        artifacts: HEADFUL_ARTIFACTS,
        explorer_policy: ManifestExplorerPolicy::RequiredAbsent,
        mandatory: true,
    }
}

pub fn validate_gui_parity_manifest(
    manifest: &[GuiSurfaceSpec],
    catalog: &[TestCase],
) -> Result<(), Vec<String>> {
    let known_cases = catalog
        .iter()
        .map(|case| case.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    let mut errors = Vec::new();
    for surface in manifest {
        if !ids.insert(surface.id) {
            errors.push(format!("duplicate-surface:{}", surface.id));
        }
        if surface.variants.is_empty() {
            errors.push(format!("missing-variants:{}", surface.id));
        }
        if surface.rules.is_empty() {
            errors.push(format!("missing-rules:{}", surface.id));
        }
        if surface.case_ids.is_empty() {
            errors.push(format!("missing-cases:{}", surface.id));
        }
        if surface.artifacts.is_empty()
            || surface
                .artifacts
                .iter()
                .any(|artifact| artifact.name.is_empty())
        {
            errors.push(format!("missing-artifacts:{}", surface.id));
        }
        for case in surface.case_ids {
            if !known_cases.contains(case) {
                errors.push(format!("unknown-case:{}:{case}", surface.id));
            }
        }
        for rule in surface.rules {
            if !valid_rule(rule) {
                errors.push(format!("invalid-rule:{}:{rule:?}", surface.id));
            }
        }
    }
    for case in catalog
        .iter()
        .filter(|case| case.tags.iter().any(|tag| tag == "gui-parity"))
    {
        if !manifest
            .iter()
            .any(|surface| surface.case_ids.contains(&case.id.as_str()))
        {
            errors.push(format!("orphan-gui-case:{}", case.id));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn valid_rule(rule: &GeometryRule) -> bool {
    match rule {
        GeometryRule::Width { region, min, max } | GeometryRule::Height { region, min, max } => {
            !region.is_empty() && min.is_finite() && max.is_finite() && *min >= 0.0 && min <= max
        }
        GeometryRule::MinTarget {
            region,
            width,
            height,
        } => !region.is_empty() && *width > 0.0 && *height > 0.0,
        GeometryRule::Contained { inner, outer } => {
            !inner.is_empty() && !outer.is_empty() && inner != outer
        }
        GeometryRule::NonOverlap { left, right } => {
            !left.is_empty() && !right.is_empty() && left != right
        }
        GeometryRule::Ratio {
            numerator,
            denominator,
            min,
            max,
        } => {
            !numerator.is_empty()
                && !denominator.is_empty()
                && numerator != denominator
                && *min >= 0.0
                && min <= max
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct GuiRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}
impl GuiRect {
    fn width(self) -> f64 {
        self.right - self.left
    }
    fn height(self) -> f64 {
        self.bottom - self.top
    }
    fn valid(self) -> bool {
        [self.left, self.top, self.right, self.bottom]
            .into_iter()
            .all(f64::is_finite)
            && self.width() >= 0.0
            && self.height() >= 0.0
    }
    fn dip(self, dpi: u32) -> Self {
        let scale = 96.0 / f64::from(dpi);
        Self {
            left: self.left * scale,
            top: self.top * scale,
            right: self.right * scale,
            bottom: self.bottom * scale,
        }
    }
    fn contains(self, other: Self) -> bool {
        other.left >= self.left
            && other.top >= self.top
            && other.right <= self.right
            && other.bottom <= self.bottom
    }
    fn overlaps(self, other: Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GuiRegion {
    pub id: String,
    pub physical: GuiRect,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GuiActionRecord {
    pub id: String,
    pub terminal: String,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GuiMeasurement {
    pub schema: String,
    pub surface_id: String,
    pub variant: String,
    pub generation: u64,
    pub dpi: u32,
    pub explorer_absent: bool,
    pub regions: Vec<GuiRegion>,
    #[serde(default)]
    pub actions: Vec<GuiActionRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuiParityFailure {
    pub surface_id: String,
    pub rule: String,
    pub expected: String,
    pub actual: String,
}

pub fn validate_gui_measurement(
    spec: &GuiSurfaceSpec,
    value: &GuiMeasurement,
) -> Result<(), Vec<GuiParityFailure>> {
    let mut failures = Vec::new();
    if value.schema != GUI_MEASUREMENT_SCHEMA {
        fail(
            &mut failures,
            spec,
            "schema",
            GUI_MEASUREMENT_SCHEMA,
            &value.schema,
        );
    }
    if value.surface_id != spec.id {
        fail(&mut failures, spec, "surface", spec.id, &value.surface_id);
    }
    if value.generation == 0 {
        fail(&mut failures, spec, "generation", "> 0", "0");
    }
    if value.dpi == 0 {
        fail(&mut failures, spec, "dpi", "> 0", "0");
        return Err(failures);
    }
    if spec.explorer_policy == ManifestExplorerPolicy::RequiredAbsent && !value.explorer_absent {
        fail(&mut failures, spec, "explorer", "absent", "present");
    }
    let regions = value
        .regions
        .iter()
        .filter(|region| region.physical.valid())
        .map(|region| (region.id.as_str(), region.physical.dip(value.dpi)))
        .collect::<BTreeMap<_, _>>();
    if regions.len() != value.regions.len() {
        fail(
            &mut failures,
            spec,
            "regions",
            "valid unique rectangles",
            "invalid or duplicate",
        );
    }
    for rule in spec.rules {
        evaluate_rule(spec, rule, &regions, &mut failures);
    }
    for control in spec.required_controls {
        if !regions.contains_key(control) {
            fail(&mut failures, spec, "control", control, "missing");
        }
    }
    for action in spec.required_actions {
        if !value
            .actions
            .iter()
            .any(|record| record.id == *action && record.terminal == "passed")
        {
            fail(&mut failures, spec, "action", action, "missing-or-failed");
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn evaluate_rule(
    spec: &GuiSurfaceSpec,
    rule: &GeometryRule,
    regions: &BTreeMap<&str, GuiRect>,
    failures: &mut Vec<GuiParityFailure>,
) {
    let range = |id: &str, value: f64, min: f64, max: f64, failures: &mut Vec<GuiParityFailure>| {
        if value < min || value > max {
            fail(
                failures,
                spec,
                id,
                &format!("{min}..{max}"),
                &format!("{value:.3}"),
            );
        }
    };
    match *rule {
        GeometryRule::Width { region, min, max } => {
            if let Some(rect) = regions.get(region) {
                range(&format!("width:{region}"), rect.width(), min, max, failures)
            } else {
                fail(failures, spec, "region", region, "missing")
            }
        }
        GeometryRule::Height { region, min, max } => {
            if let Some(rect) = regions.get(region) {
                range(
                    &format!("height:{region}"),
                    rect.height(),
                    min,
                    max,
                    failures,
                )
            } else {
                fail(failures, spec, "region", region, "missing")
            }
        }
        GeometryRule::MinTarget {
            region,
            width,
            height,
        } => {
            if let Some(rect) = regions.get(region) {
                if rect.width() < width || rect.height() < height {
                    fail(
                        failures,
                        spec,
                        &format!("target:{region}"),
                        &format!(">={width}x{height}"),
                        &format!("{:.3}x{:.3}", rect.width(), rect.height()),
                    );
                }
            } else {
                fail(failures, spec, "region", region, "missing")
            }
        }
        GeometryRule::Contained { inner, outer } => {
            match (regions.get(inner), regions.get(outer)) {
                (Some(inner_rect), Some(outer_rect)) if outer_rect.contains(*inner_rect) => {}
                (Some(_), Some(_)) => fail(
                    failures,
                    spec,
                    &format!("contained:{inner}"),
                    outer,
                    "outside",
                ),
                _ => fail(
                    failures,
                    spec,
                    "region",
                    &format!("{inner}|{outer}"),
                    "missing",
                ),
            }
        }
        GeometryRule::NonOverlap { left, right } => match (regions.get(left), regions.get(right)) {
            (Some(a), Some(b)) if a.overlaps(*b) => fail(
                failures,
                spec,
                &format!("overlap:{left}:{right}"),
                "non-overlap",
                "overlap",
            ),
            _ => {}
        },
        GeometryRule::Ratio {
            numerator,
            denominator,
            min,
            max,
        } => match (regions.get(numerator), regions.get(denominator)) {
            (Some(a), Some(b)) if b.width() > 0.0 => range(
                &format!("ratio:{numerator}:{denominator}"),
                a.width() / b.width(),
                min,
                max,
                failures,
            ),
            _ => fail(
                failures,
                spec,
                "region",
                &format!("{numerator}|{denominator}"),
                "missing-or-zero",
            ),
        },
    }
}

fn fail(
    failures: &mut Vec<GuiParityFailure>,
    spec: &GuiSurfaceSpec,
    rule: &str,
    expected: &str,
    actual: &str,
) {
    failures.push(GuiParityFailure {
        surface_id: spec.id.into(),
        rule: rule.into(),
        expected: expected.into(),
        actual: actual.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    #[test]
    fn first_wave_manifest_is_unique_valid_and_catalog_closed() {
        validate_gui_parity_manifest(&gui_parity_manifest(), &catalog()).unwrap();
    }

    #[test]
    fn normalized_measurement_converts_once_and_reports_exact_failures() {
        let spec = GuiSurfaceSpec {
            id: "fixture",
            owner: "test",
            reference_family: "test",
            case_ids: &["unit-utit"],
            variants: DEFAULT_VARIANTS,
            rules: &[
                GeometryRule::Width {
                    region: "window",
                    min: 100.0,
                    max: 100.0,
                },
                GeometryRule::Contained {
                    inner: "window",
                    outer: "monitor",
                },
                GeometryRule::MinTarget {
                    region: "control",
                    width: 44.0,
                    height: 40.0,
                },
            ],
            required_controls: &["window", "control"],
            required_actions: &["pointer"],
            artifacts: UNIT_ARTIFACTS,
            explorer_policy: ManifestExplorerPolicy::RequiredAbsent,
            mandatory: true,
        };
        let valid = GuiMeasurement {
            schema: GUI_MEASUREMENT_SCHEMA.into(),
            surface_id: "fixture".into(),
            variant: "dpi-192".into(),
            generation: 1,
            dpi: 192,
            explorer_absent: true,
            regions: vec![
                GuiRegion {
                    id: "monitor".into(),
                    physical: GuiRect {
                        left: -400.0,
                        top: 0.0,
                        right: 400.0,
                        bottom: 600.0,
                    },
                },
                GuiRegion {
                    id: "window".into(),
                    physical: GuiRect {
                        left: -200.0,
                        top: 0.0,
                        right: 0.0,
                        bottom: 200.0,
                    },
                },
                GuiRegion {
                    id: "control".into(),
                    physical: GuiRect {
                        left: -100.0,
                        top: 20.0,
                        right: -12.0,
                        bottom: 100.0,
                    },
                },
            ],
            actions: vec![GuiActionRecord {
                id: "pointer".into(),
                terminal: "passed".into(),
            }],
        };
        validate_gui_measurement(&spec, &valid).unwrap();
        let mut invalid = valid.clone();
        invalid.dpi = 96;
        invalid.explorer_absent = false;
        let errors = validate_gui_measurement(&spec, &invalid).unwrap_err();
        assert!(errors.iter().any(|error| error.rule == "width:window"));
        assert!(errors.iter().any(|error| error.rule == "explorer"));
    }

    #[test]
    fn malformed_stale_overlap_ratio_and_zero_dpi_fail_deterministically() {
        let spec = &gui_parity_manifest()[0];
        let value = GuiMeasurement {
            schema: "bad".into(),
            surface_id: spec.id.into(),
            variant: "bad".into(),
            generation: 0,
            dpi: 0,
            explorer_absent: false,
            regions: vec![],
            actions: vec![],
        };
        let errors = validate_gui_measurement(spec, &value).unwrap_err();
        assert_eq!(errors[0].rule, "schema");
        assert!(errors.iter().any(|error| error.rule == "generation"));
        assert!(errors.iter().any(|error| error.rule == "dpi"));
    }
}
