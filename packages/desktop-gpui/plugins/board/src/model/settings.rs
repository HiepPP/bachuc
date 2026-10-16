//! The three host settings the Board reads: `thinking-orb` (`shared/orb.ts:36-45`),
//! `project-colors` (`shared/project-colors.ts`), and `board-size` (`shared/board-size.ts`).

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const ORB_SETTINGS_ID: &str = "thinking-orb";
pub(crate) const PROJECT_COLORS_ID: &str = "project-colors";
pub(crate) const BOARD_SIZE_ID: &str = "board-size";

pub(crate) const BOARD_SIZE_DEFAULT: u16 = 100;
pub(crate) const BOARD_SIZE_MIN: u16 = 10;
pub(crate) const BOARD_SIZE_MAX: u16 = 200;
pub(crate) const BOARD_SIZE_STEP: u16 = 10;
/// 100% renders what the original 70% did.
const BASE_SCALE: f32 = 0.7;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum OrbState {
    #[default]
    Solving,
    Working,
    Searching,
    Listening,
    Weaving,
    Composing,
    Connecting,
}

impl OrbState {
    fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "solving" => Self::Solving,
            "working" => Self::Working,
            "searching" => Self::Searching,
            "listening" => Self::Listening,
            "weaving" => Self::Weaving,
            "composing" => Self::Composing,
            "connecting" => Self::Connecting,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OrbSettings {
    pub enabled: bool,
    pub state: OrbState,
    pub avatar_opacity: u8,
}

impl Default for OrbSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            state: OrbState::Solving,
            avatar_opacity: 100,
        }
    }
}

/// The saved orb settings; each missing or invalid field keeps its schema default.
pub(crate) fn orb_settings(values: &Value) -> OrbSettings {
    let defaults = OrbSettings::default();
    OrbSettings {
        enabled: values["enabled"].as_bool().unwrap_or(defaults.enabled),
        state: values["state"]
            .as_str()
            .and_then(OrbState::from_id)
            .unwrap_or(defaults.state),
        avatar_opacity: values["avatarOpacity"]
            .as_u64()
            .filter(|value| (20..=100).contains(value) && value % 10 == 0)
            .map_or(defaults.avatar_opacity, |value| value as u8),
    }
}

/// thinking-orbs ships tuned designs at 20, 32, and 64 px; other sizes scale the nearest.
#[allow(dead_code)] // TASK-193's orb sizes with it.
pub(crate) fn orb_preset(target: f32) -> (u16, f32) {
    let size = [20u16, 32, 64]
        .into_iter()
        .reduce(|best, preset| {
            if (f32::from(preset) - target).abs() < (f32::from(best) - target).abs() {
                preset
            } else {
                best
            }
        })
        .unwrap_or(32);
    (size, target / f32::from(size))
}

/// The saved `hues` of `project-colors`.
pub(crate) fn decode_hues(values: &Value) -> BTreeMap<String, u16> {
    values["hues"]
        .as_object()
        .map(|hues| {
            hues.iter()
                .filter_map(|(id, hue)| {
                    let hue = hue.as_u64().filter(|hue| *hue < 360)?;
                    Some((id.clone(), hue as u16))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn hues_values(hues: &BTreeMap<String, u16>) -> Value {
    json!({ "hues": hues })
}

/// `allocateColors`: each new project takes the hue farthest from every used hue; existing
/// hues never change.
pub(crate) fn allocate_colors(
    existing: &BTreeMap<String, u16>,
    ids: &[String],
) -> BTreeMap<String, u16> {
    let missing: BTreeSet<&String> = ids
        .iter()
        .filter(|id| !existing.contains_key(*id))
        .collect();
    let mut result = existing.clone();
    let mut used: Vec<u16> = existing.values().copied().collect();
    for id in missing {
        let (mut best_hue, mut best_distance) = (0, -1);
        for hue in 0..360u16 {
            let distance = used.iter().fold(180, |minimum, &other| {
                let difference = (i32::from(hue) - i32::from(other)).abs();
                minimum.min(difference).min(360 - difference)
            });
            if distance > best_distance {
                best_hue = hue;
                best_distance = distance;
            }
        }
        result.insert(id.clone(), best_hue);
        used.push(best_hue);
    }
    result
}

pub(crate) fn clamp_board_size(size: i64) -> u16 {
    size.clamp(i64::from(BOARD_SIZE_MIN), i64::from(BOARD_SIZE_MAX)) as u16
}

/// Every Runs measure multiplies by this.
pub(crate) fn board_scale(size: u16) -> f32 {
    f32::from(size) / 100. * BASE_SCALE
}

pub(crate) fn decode_size(values: &Value) -> u16 {
    values["size"]
        .as_i64()
        .map_or(BOARD_SIZE_DEFAULT, clamp_board_size)
}

/// The size control's save queue (`BoardPage`, `page.tsx:1710-1740`): one write at a time,
/// the newest unsaved choice wins, and a failed value retries only after a new pick.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SizeSaver {
    pending: Option<u16>,
    saving: bool,
    failed: Option<u16>,
}

impl SizeSaver {
    pub(crate) fn pick(&mut self, size: i64) {
        self.failed = None;
        self.pending = Some(clamp_board_size(size));
    }

    /// The size on screen: the pending choice, else the stored one.
    pub(crate) fn size(&self, stored: Option<u16>) -> u16 {
        self.pending.or(stored).unwrap_or(BOARD_SIZE_DEFAULT)
    }

    /// The value to write now, if any. `stored` is `None` until the settings are ready;
    /// `busy` is the settings entity's own save.
    pub(crate) fn next_save(&mut self, stored: Option<u16>, busy: bool) -> Option<u16> {
        let pending = self.pending?;
        let stored = stored?;
        if busy || self.saving {
            return None;
        }
        if pending == stored {
            self.pending = None;
            return None;
        }
        if self.failed == Some(pending) {
            return None;
        }
        self.saving = true;
        Some(pending)
    }

    pub(crate) fn saved(&mut self, size: u16, ok: bool) {
        self.saving = false;
        self.failed = (!ok).then_some(size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hues(pairs: &[(&str, u16)]) -> BTreeMap<String, u16> {
        pairs
            .iter()
            .map(|(id, hue)| (id.to_string(), *hue))
            .collect()
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|id| id.to_string()).collect()
    }

    #[test]
    fn a_new_project_takes_the_farthest_hue_and_existing_hues_never_change() {
        assert_eq!(
            allocate_colors(&hues(&[("a", 0)]), &ids(&["b"])),
            hues(&[("a", 0), ("b", 180)])
        );
        let initial = allocate_colors(&BTreeMap::new(), &ids(&["b", "a"]));
        assert_eq!(initial, hues(&[("a", 0), ("b", 180)]));
        let restored = decode_hues(&hues_values(&initial));
        let next = allocate_colors(&restored, &ids(&["c", "a", "b"]));
        assert_eq!(next, hues(&[("a", 0), ("b", 180), ("c", 90)]));
        assert_eq!(allocate_colors(&next, &ids(&["c", "b", "a"])), next);
        assert_eq!(
            allocate_colors(&BTreeMap::new(), &ids(&["b", "a", "b"])),
            initial
        );
        assert_eq!(
            allocate_colors(&hues(&[("a", 0), ("b", 180)]), &ids(&["d", "c"])),
            hues(&[("a", 0), ("b", 180), ("c", 90), ("d", 270)])
        );
    }

    #[test]
    fn the_board_size_clamps_and_scales_from_seventy_percent() {
        assert_eq!(clamp_board_size(5), 10);
        assert_eq!(clamp_board_size(250), 200);
        assert_eq!(clamp_board_size(70), 70);
        assert_eq!(board_scale(100), 0.7);
        assert_eq!(board_scale(50), 0.35);
        assert_eq!(board_scale(200), 1.4);
        assert_eq!(decode_size(&json!({})), 100);
        assert_eq!(decode_size(&json!({"size": 70})), 70);
        assert_eq!(decode_size(&json!({"size": 900})), 200);
    }

    #[test]
    fn the_size_saver_writes_one_value_at_a_time_and_keeps_the_newest_choice() {
        let mut saver = SizeSaver::default();
        assert_eq!(saver.size(None), 100);
        saver.pick(110);
        assert_eq!(saver.size(Some(100)), 110);
        // Not before the settings load.
        assert_eq!(saver.next_save(None, false), None);
        assert_eq!(saver.next_save(Some(100), false), Some(110));
        // One write at a time; the newest pick waits.
        saver.pick(120);
        assert_eq!(saver.next_save(Some(100), false), None);
        saver.saved(110, true);
        assert_eq!(saver.next_save(Some(110), true), None);
        assert_eq!(saver.next_save(Some(110), false), Some(120));
        // A failed value retries only after a new pick.
        saver.saved(120, false);
        assert_eq!(saver.next_save(Some(110), false), None);
        saver.pick(120);
        assert_eq!(saver.next_save(Some(110), false), Some(120));
        saver.saved(120, true);
        // Stored now matches: the pending choice clears.
        assert_eq!(saver.next_save(Some(120), false), None);
        assert_eq!(saver.size(Some(120)), 120);
        saver.pick(-5);
        assert_eq!(saver.size(Some(120)), 10);
    }

    #[test]
    fn orb_settings_default_to_a_shown_solving_orb_over_an_opaque_avatar() {
        assert_eq!(orb_settings(&json!({})), OrbSettings::default());
        assert_eq!(
            orb_settings(&json!({"enabled": false, "state": "weaving", "avatarOpacity": 50})),
            OrbSettings {
                enabled: false,
                state: OrbState::Weaving,
                avatar_opacity: 50,
            }
        );
        assert_eq!(
            orb_settings(&json!({"avatarOpacity": 15})).avatar_opacity,
            100
        );
        assert_eq!(
            orb_settings(&json!({"avatarOpacity": 55})).avatar_opacity,
            100
        );
        assert_eq!(
            orb_settings(&json!({"state": "breathing"})).state,
            OrbState::Solving
        );
    }

    #[test]
    fn the_orb_preset_is_the_nearest_tuned_size() {
        assert_eq!(orb_preset(36.), (32, 36. / 32.));
        assert_eq!(orb_preset(12.), (20, 12. / 20.));
        assert_eq!(orb_preset(72.), (64, 72. / 64.));
        assert_eq!(orb_preset(26.).0, 20);
        assert_eq!(orb_preset(50.).0, 64);
    }
}
