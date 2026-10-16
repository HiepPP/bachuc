//! The toolbar density rule of `packages/app/src/composer/agent-controls/layout.ts`: how much of
//! each control shows for the width the toolbar has.

/// `COMPOSER_TOOLBAR_GEOMETRY`.
pub const CONTROL_SIZE: f32 = 28.;
pub const CONTROL_GAP: f32 = 4.;
pub const ICON_LABEL_GAP: f32 = 4.;
pub const LABEL_PADDING: f32 = 8.;

/// `DENSITY_HYSTERESIS`: a width must pass a floor by this much before the density changes
/// back, so a toolbar at the edge does not flip on every frame.
const HYSTERESIS: f32 = 12.;
/// `estimateLabelWidth`: 7 px per character at font scale 1.
const LABEL_CHAR_WIDTH: f32 = 7.;

/// `ComposerControlDensity`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    #[default]
    Full,
    Condensed,
    Tight,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FeaturePresence {
    Toggle,
    /// A select with the label it shows.
    Select {
        label: String,
    },
}

/// `ComposerControlPresence` at font scale 1: the desktop window has no font scale.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ControlPresence {
    pub has_model: bool,
    pub has_thinking: bool,
    pub has_mode: bool,
    pub features: Vec<FeaturePresence>,
}

/// `ComposerControlPresentation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Presentation {
    pub show_carets: bool,
    pub show_mode_label: bool,
    /// The features fold into one Settings2 button.
    pub aggregate_features: bool,
}

fn sum_widths(widths: &[f32]) -> f32 {
    if widths.is_empty() {
        return 0.;
    }
    widths.iter().sum::<f32>() + (widths.len() - 1) as f32 * CONTROL_GAP
}

fn feature_width(feature: &FeaturePresence) -> f32 {
    match feature {
        FeaturePresence::Toggle => CONTROL_SIZE,
        FeaturePresence::Select { label } => {
            CONTROL_SIZE
                + ICON_LABEL_GAP
                + LABEL_PADDING * 2.
                + label.chars().count() as f32 * LABEL_CHAR_WIDTH
        }
    }
}

/// `resolveFullFloor`.
pub fn full_floor(controls: &ControlPresence) -> f32 {
    let mut widths = Vec::new();
    if controls.has_model {
        widths.push(50. + 70.);
    }
    if controls.has_thinking {
        widths.push(54. + 48.);
    }
    if controls.has_mode {
        widths.push(54. + 96.);
    }
    widths.extend(controls.features.iter().map(feature_width));
    sum_widths(&widths)
}

/// `resolveCondensedFloor`.
pub fn condensed_floor(controls: &ControlPresence) -> f32 {
    let mut widths = Vec::new();
    if controls.has_model {
        widths.push(36. + 60.);
    }
    if controls.has_thinking {
        widths.push(36. + 48.);
    }
    if controls.has_mode {
        widths.push(36. + 96.);
    }
    if !controls.features.is_empty() {
        widths.push(CONTROL_SIZE);
    }
    sum_widths(&widths)
}

/// `resolveComposerControlDensity`: the density for `available` px, starting from `current`.
pub fn resolve_density(available: f32, current: Density, controls: &ControlPresence) -> Density {
    let full = full_floor(controls);
    let condensed = condensed_floor(controls);
    match current {
        Density::Full => {
            if available >= full - HYSTERESIS {
                Density::Full
            } else if available >= condensed {
                Density::Condensed
            } else {
                Density::Tight
            }
        }
        Density::Condensed => {
            if available >= full + HYSTERESIS {
                Density::Full
            } else if available < condensed - HYSTERESIS {
                Density::Tight
            } else {
                Density::Condensed
            }
        }
        Density::Tight => {
            if available >= full + HYSTERESIS {
                Density::Full
            } else if available >= condensed + HYSTERESIS {
                Density::Condensed
            } else {
                Density::Tight
            }
        }
    }
}

/// `resolveComposerControlPresentation`.
pub fn presentation(density: Density) -> Presentation {
    match density {
        Density::Full => Presentation {
            show_carets: true,
            show_mode_label: true,
            aggregate_features: false,
        },
        Density::Condensed => Presentation {
            show_carets: false,
            show_mode_label: true,
            aggregate_features: true,
        },
        Density::Tight => Presentation {
            show_carets: false,
            show_mode_label: false,
            aggregate_features: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controls() -> ControlPresence {
        ControlPresence {
            has_model: true,
            has_thinking: true,
            has_mode: true,
            features: vec![
                FeaturePresence::Toggle,
                FeaturePresence::Select {
                    label: "High".into(),
                },
            ],
        }
    }

    #[test]
    fn the_floors_add_each_control_and_the_gaps() {
        let controls = controls();
        // 120 + 102 + 150 + 28 + (28 + 4 + 16 + 28) and 4 gaps of 4.
        assert_eq!(full_floor(&controls), 120. + 102. + 150. + 28. + 76. + 16.);
        // 96 + 84 + 132 + 28 and 3 gaps of 4.
        assert_eq!(condensed_floor(&controls), 96. + 84. + 132. + 28. + 12.);
        assert_eq!(full_floor(&ControlPresence::default()), 0.);
    }

    #[test]
    fn full_drops_to_condensed_below_the_full_floor_minus_the_hysteresis() {
        let controls = controls();
        let full = full_floor(&controls);
        let condensed = condensed_floor(&controls);
        assert_eq!(
            resolve_density(full - 12., Density::Full, &controls),
            Density::Full
        );
        assert_eq!(
            resolve_density(full - 12.5, Density::Full, &controls),
            Density::Condensed
        );
        assert_eq!(
            resolve_density(condensed - 0.5, Density::Full, &controls),
            Density::Tight
        );
    }

    #[test]
    fn condensed_needs_the_hysteresis_to_change_either_way() {
        let controls = controls();
        let full = full_floor(&controls);
        let condensed = condensed_floor(&controls);
        assert_eq!(
            resolve_density(full + 11., Density::Condensed, &controls),
            Density::Condensed
        );
        assert_eq!(
            resolve_density(full + 12., Density::Condensed, &controls),
            Density::Full
        );
        assert_eq!(
            resolve_density(condensed - 12., Density::Condensed, &controls),
            Density::Condensed
        );
        assert_eq!(
            resolve_density(condensed - 12.5, Density::Condensed, &controls),
            Density::Tight
        );
    }

    #[test]
    fn tight_climbs_only_past_a_floor_plus_the_hysteresis() {
        let controls = controls();
        let full = full_floor(&controls);
        let condensed = condensed_floor(&controls);
        assert_eq!(
            resolve_density(condensed + 11., Density::Tight, &controls),
            Density::Tight
        );
        assert_eq!(
            resolve_density(condensed + 12., Density::Tight, &controls),
            Density::Condensed
        );
        assert_eq!(
            resolve_density(full + 12., Density::Tight, &controls),
            Density::Full
        );
    }

    #[test]
    fn each_density_shows_less() {
        assert_eq!(
            presentation(Density::Full),
            Presentation {
                show_carets: true,
                show_mode_label: true,
                aggregate_features: false,
            }
        );
        let condensed = presentation(Density::Condensed);
        assert!(
            !condensed.show_carets && condensed.show_mode_label && condensed.aggregate_features
        );
        let tight = presentation(Density::Tight);
        assert!(!tight.show_carets && !tight.show_mode_label && tight.aggregate_features);
    }
}
