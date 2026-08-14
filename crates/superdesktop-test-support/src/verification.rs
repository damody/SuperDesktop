#[derive(Clone, Debug, PartialEq)]
pub struct VisualContract {
    pub base_tolerance_px: f32,
    pub minimum_ssim: f32,
    pub masks: Vec<[u32; 4]>,
    pub expected_states: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateGeometry {
    pub dpi: u32,
    pub deltas_px: Vec<f32>,
    pub ssim: f32,
    pub states: Vec<String>,
    pub hit_target_px: f32,
}

impl VisualContract {
    pub fn scaled_tolerance(&self, dpi: u32) -> f32 {
        (self.base_tolerance_px * dpi as f32 / 96.0).round()
    }

    pub fn validate(&self, candidate: &CandidateGeometry) -> Result<(), &'static str> {
        let tolerance = self.scaled_tolerance(candidate.dpi);
        if candidate.ssim < self.minimum_ssim {
            return Err("ssim");
        }
        if candidate
            .deltas_px
            .iter()
            .any(|delta| delta.abs() > tolerance)
        {
            return Err("geometry");
        }
        if candidate.states != self.expected_states {
            return Err("state");
        }
        if candidate.hit_target_px < (40.0 * candidate.dpi as f32 / 96.0).round() {
            return Err("hit-target");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocaleFixture {
    pub locale: &'static str,
    pub labels: Vec<&'static str>,
    pub right_to_left: bool,
}

impl LocaleFixture {
    pub fn validate(&self) -> Result<Vec<usize>, &'static str> {
        if self.labels.is_empty()
            || self.labels.iter().any(|label| {
                label.is_empty() || label.contains('\u{fffd}') || label.chars().count() > 32
            })
        {
            return Err("missing-glyph-or-unsafe-truncation");
        }
        let mut reading_order = (0..self.labels.len()).collect::<Vec<_>>();
        if self.right_to_left {
            reading_order.reverse();
        }
        Ok(reading_order)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ImeFixture {
    composition: String,
    committed: String,
    focused: bool,
}

impl ImeFixture {
    pub fn focus(&mut self) {
        self.focused = true;
    }
    pub fn compose(&mut self, value: &str) -> Result<(), &'static str> {
        if !self.focused {
            return Err("ime-without-focus");
        }
        self.composition = value.to_owned();
        Ok(())
    }
    pub fn commit(&mut self) {
        self.committed.push_str(&self.composition);
        self.composition.clear();
    }
    pub fn cancel(&mut self) {
        self.composition.clear();
    }
    pub fn snapshot(&self) -> (&str, &str, bool) {
        (&self.composition, &self.committed, self.focused)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ResourceSeries {
    pub samples: Vec<u64>,
    pub maximum: u64,
    pub stable_tail_delta: u64,
}

impl ResourceSeries {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.samples.len() < 5 || self.samples.iter().any(|value| *value > self.maximum) {
            return Err("resource-bound");
        }
        let tail = &self.samples[self.samples.len() - 3..];
        let (min, max) = (
            *tail.iter().min().expect("tail nonempty"),
            *tail.iter().max().expect("tail nonempty"),
        );
        if max - min > self.stable_tail_delta {
            return Err("resource-not-stable");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerformanceSamples {
    pub cold_start_ms: Vec<f64>,
    pub idle_cpu_percent: Vec<f64>,
    pub event_latency_ms: Vec<f64>,
    pub working_set_bytes: Vec<u64>,
}

impl PerformanceSamples {
    fn percentile(values: &[f64], percentile: f64) -> f64 {
        let mut values = values.to_vec();
        values.sort_by(f64::total_cmp);
        let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
        values[index]
    }
    pub fn validate(&self) -> Result<(f64, f64, f64, u64), &'static str> {
        if self.cold_start_ms.is_empty()
            || self.idle_cpu_percent.is_empty()
            || self.event_latency_ms.is_empty()
            || self.working_set_bytes.is_empty()
        {
            return Err("missing-samples");
        }
        let cold = self.cold_start_ms.iter().copied().fold(0.0, f64::max);
        let idle = Self::percentile(&self.idle_cpu_percent, 0.5);
        let latency = Self::percentile(&self.event_latency_ms, 0.95);
        let working_set = *self.working_set_bytes.iter().max().expect("nonempty");
        if cold > 2_000.0 || idle >= 0.5 || latency >= 100.0 || working_set >= 150 * 1024 * 1024 {
            return Err("performance-threshold");
        }
        Ok((cold, idle, latency, working_set))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> VisualContract {
        VisualContract {
            base_tolerance_px: 2.0,
            minimum_ssim: 0.95,
            masks: vec![[1800, 0, 120, 80], [1920, 0, 80, 80]],
            expected_states: vec!["start".into(), "superexplorer".into(), "status".into()],
        }
    }

    #[test]
    fn five_dpi_visual_geometry_and_hit_target_matrix_passes() {
        for dpi in [96, 120, 144, 168, 192] {
            let candidate = CandidateGeometry {
                dpi,
                deltas_px: vec![0.0, 1.0, contract().scaled_tolerance(dpi)],
                ssim: 0.97,
                states: contract().expected_states.clone(),
                hit_target_px: (40.0 * dpi as f32 / 96.0).round(),
            };
            assert_eq!(contract().validate(&candidate), Ok(()));
        }
    }

    #[test]
    fn visual_contract_rejects_ssim_geometry_state_and_hit_target_drift() {
        let valid = CandidateGeometry {
            dpi: 96,
            deltas_px: vec![0.0],
            ssim: 0.96,
            states: contract().expected_states.clone(),
            hit_target_px: 40.0,
        };
        for expected in ["ssim", "geometry", "state", "hit-target"] {
            let mut value = valid.clone();
            match expected {
                "ssim" => value.ssim = 0.94,
                "geometry" => value.deltas_px = vec![3.0],
                "state" => value.states.reverse(),
                "hit-target" => value.hit_target_px = 39.0,
                _ => unreachable!(),
            }
            assert_eq!(contract().validate(&value), Err(expected));
        }
    }

    #[test]
    fn zh_tw_english_zh_cn_fallback_and_rtl_are_layout_safe() {
        for fixture in [
            LocaleFixture {
                locale: "zh-TW",
                labels: vec!["開始", "檔案總管", "設定"],
                right_to_left: false,
            },
            LocaleFixture {
                locale: "en",
                labels: vec!["Start", "SuperExplorer", "Settings"],
                right_to_left: false,
            },
            LocaleFixture {
                locale: "zh-CN",
                labels: vec!["开始", "文件资源管理器", "设置"],
                right_to_left: false,
            },
        ] {
            assert_eq!(fixture.validate(), Ok(vec![0, 1, 2]));
        }
        let rtl = LocaleFixture {
            locale: "ar",
            labels: vec!["ابدأ", "المستكشف", "الإعدادات"],
            right_to_left: true,
        };
        assert_eq!(rtl.validate(), Ok(vec![2, 1, 0]));
    }

    #[test]
    fn ime_composition_commit_cancel_preserve_focus() {
        let mut ime = ImeFixture::default();
        assert_eq!(ime.compose("注"), Err("ime-without-focus"));
        ime.focus();
        ime.compose("注音").unwrap();
        ime.commit();
        assert_eq!(ime.snapshot(), ("", "注音", true));
        ime.compose("取消").unwrap();
        ime.cancel();
        assert_eq!(ime.snapshot(), ("", "注音", true));
    }

    #[test]
    fn every_resource_counter_has_an_independent_bound_and_stable_tail() {
        for series in [
            ResourceSeries {
                samples: vec![20, 24, 25, 25, 24],
                maximum: 32,
                stable_tail_delta: 1,
            },
            ResourceSeries {
                samples: vec![2, 3, 3, 3, 3],
                maximum: 8,
                stable_tail_delta: 0,
            },
            ResourceSeries {
                samples: vec![100, 110, 111, 110, 111],
                maximum: 128,
                stable_tail_delta: 1,
            },
            ResourceSeries {
                samples: vec![5, 6, 6, 6, 6],
                maximum: 16,
                stable_tail_delta: 0,
            },
            ResourceSeries {
                samples: vec![4, 5, 5, 5, 5],
                maximum: 16,
                stable_tail_delta: 0,
            },
        ] {
            assert_eq!(series.validate(), Ok(()));
        }
    }

    #[test]
    fn fixed_performance_thresholds_use_raw_samples() {
        let samples = PerformanceSamples {
            cold_start_ms: vec![22.0, 18.0, 19.0],
            idle_cpu_percent: vec![0.02, 0.01, 0.03, 0.02, 0.01],
            event_latency_ms: vec![
                1.0, 2.0, 1.5, 3.0, 2.5, 2.0, 1.0, 2.0, 1.0, 2.0, 3.5, 2.2, 1.8, 1.4, 1.6, 2.4,
                2.6, 1.3, 1.2, 2.1,
            ],
            working_set_bytes: vec![20 * 1024 * 1024, 21 * 1024 * 1024, 20 * 1024 * 1024],
        };
        assert!(samples.validate().is_ok());
    }
}
