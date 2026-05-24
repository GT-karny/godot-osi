//! OpenDRIVE signal classification.
//!
//! Maps an OpenDRIVE `<signal>`'s `(type, subtype)` to a human- and
//! engine-meaningful classification (category, sub-category, lamp colour /
//! arrow direction, OSI type name, icon key, localized labels).
//!
//! The data is sourced from the OpenDRIVE `Signal_Base_catalog` plus esmini's
//! authoritative `traffic_light_type_map`, baked into [`CATALOG_TSV`] at build
//! time by `tools/gen_signal_catalog.py`. This module is pure data: no FFI and
//! no engine dependencies, so it is unit-testable on its own.
//!
//! Scope: country `"OpenDRIVE"` (the standard catalogue, `1000xxx` codes plus
//! tram signals). Country-specific catalogues (e.g. German StVO) are out of
//! scope for now but can be added as additional rows without code changes.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Catalogue data, regenerate with `tools/gen_signal_catalog.py`.
const CATALOG_TSV: &str = include_str!("signal_catalog.tsv");

/// Classification of a single OpenDRIVE signal.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SignalClass {
    /// `traffic_light` | `road_marking` | `tram_signal` (extensible).
    pub category: String,
    /// e.g. `vehicle` | `pedestrian` | `bicycle` | `vehicle_arrow` | `tram`.
    pub subcategory: String,
    /// Lit lamp colour for single-aspect entries: `red` | `yellow` | `green`;
    /// `multi` for a full multi-aspect head; `none` otherwise.
    pub color: String,
    /// Arrow direction for directional lights: `left` | `right` | `straight` |
    /// `straight_left` | `diag_right` | `down` | `left_right` | `cross` | …,
    /// or `none`.
    pub arrow: String,
    /// OSI `TrafficSign.MainSign.Classification.Type` enum name (often
    /// `TYPE_UNKNOWN` for OpenDRIVE-country lights, which are not main signs).
    pub osi_type_name: String,
    /// Number of lamps in the signal head (0 for markings / unknown).
    pub nr_lamps: u32,
    /// Icon key (file stem under `addons/godot_osi/icons/signals/`), or empty
    /// when no catalogue artwork exists for the entry.
    pub icon: String,
    /// English label.
    pub label_en: String,
    /// Japanese label.
    pub label_ja: String,
    /// `false` if no catalogue row matched (all other fields are defaults).
    pub matched: bool,
}

/// Parse the embedded TSV once into a `(type, subtype) -> class` table.
fn table() -> &'static HashMap<(String, String), SignalClass> {
    static TABLE: OnceLock<HashMap<(String, String), SignalClass>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut map = HashMap::new();
        for line in CATALOG_TSV.lines() {
            if line.is_empty() || line.starts_with('#') || line.starts_with("type\t") {
                continue;
            }
            let f: Vec<&str> = line.split('\t').collect();
            if f.len() < 11 {
                continue;
            }
            let key = (f[0].to_string(), f[1].to_string());
            map.insert(
                key,
                SignalClass {
                    category: f[2].to_string(),
                    subcategory: f[3].to_string(),
                    color: f[4].to_string(),
                    arrow: f[5].to_string(),
                    osi_type_name: f[6].to_string(),
                    nr_lamps: f[7].parse().unwrap_or(0),
                    icon: f[8].to_string(),
                    label_en: f[9].to_string(),
                    label_ja: f[10].to_string(),
                    matched: true,
                },
            );
        }
        map
    })
}

/// Normalize an OpenDRIVE `type` for lookup: strip dots and whitespace so
/// `"1.000.001"` and `"1000001"` both resolve to `"1000001"`.
fn norm_type(t: &str) -> String {
    t.chars()
        .filter(|c| *c != '.' && !c.is_whitespace())
        .collect()
}

/// Normalize a `subtype`: empty, `-1`, `-` and `none` all map to `"none"`.
fn norm_subtype(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() || s == "-1" || s == "-" || s == "none" {
        "none".to_string()
    } else {
        s.to_string()
    }
}

/// Classify a signal by its OpenDRIVE `type`/`subtype`.
///
/// Tries an exact `(type, subtype)` match first, then falls back to the
/// `type`-only row (`subtype = none`). Returns an unmatched [`SignalClass`]
/// (`matched == false`) when nothing in the catalogue applies.
pub fn classify(sign_type: &str, subtype: &str) -> SignalClass {
    let t = norm_type(sign_type);
    let s = norm_subtype(subtype);
    let tbl = table();
    if let Some(c) = tbl.get(&(t.clone(), s)) {
        return c.clone();
    }
    if let Some(c) = tbl.get(&(t, "none".to_string())) {
        return c.clone();
    }
    SignalClass::default()
}

/// Resolve a raw OSI `TrafficSign.MainSign.Classification.Type` integer (as
/// reported by esmini's `Signal::GetOSIType`) to its enum name, e.g.
/// `49 -> "TYPE_SPEED_LIMIT_BEGIN"`. Returns `"TYPE_UNKNOWN"` for values that
/// are not part of the enum.
pub fn osi_type_name_from_int(osi_type: i32) -> String {
    use osi_types::osi3::traffic_sign::main_sign::classification::Type as MainType;
    MainType::try_from(osi_type)
        .map(|t| t.as_str_name().to_string())
        .unwrap_or_else(|_| "TYPE_UNKNOWN".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vehicle_three_aspect() {
        let c = classify("1000001", "");
        assert!(c.matched);
        assert_eq!(c.category, "traffic_light");
        assert_eq!(c.subcategory, "vehicle");
        assert_eq!(c.nr_lamps, 3);
        assert_eq!(c.icon, "odr_1000001_none");
    }

    #[test]
    fn dotted_type_normalizes() {
        // The dotted catalogue form resolves the same as the dotless xodr form.
        assert_eq!(classify("1.000.001", "-1"), classify("1000001", ""));
    }

    #[test]
    fn pedestrian_red_state() {
        let c = classify("1000002", "10");
        assert_eq!(c.subcategory, "pedestrian");
        assert_eq!(c.color, "red");
    }

    #[test]
    fn arrow_direction_and_color() {
        let c = classify("1000020", "10");
        assert_eq!(c.subcategory, "vehicle_arrow");
        assert_eq!(c.arrow, "left");
        assert_eq!(c.color, "red");
    }

    #[test]
    fn subtype_fallback_to_type_only() {
        // An uncatalogued subtype falls back to the type-only entry.
        let c = classify("1000001", "999");
        assert!(c.matched);
        assert_eq!(c.nr_lamps, 3);
    }

    #[test]
    fn pedestrian_crossing_marking() {
        let c = classify("1000003", "-1");
        assert_eq!(c.category, "road_marking");
        assert_eq!(c.subcategory, "pedestrian_crossing");
    }

    #[test]
    fn unknown_returns_unmatched() {
        let c = classify("999999", "");
        assert!(!c.matched);
        assert!(c.category.is_empty());
    }

    #[test]
    fn osi_int_to_name() {
        // 0 is TYPE_UNKNOWN in the OSI enum.
        assert_eq!(osi_type_name_from_int(0), "TYPE_UNKNOWN");
    }

    // End-to-end: the type/subtype strings a real OpenDRIVE map yields must
    // classify. `multi_intersections.xodr` uses country "OpenDRIVE" lights
    // (1000001/1000002) and a crossing marking (1000003). Skips gracefully if
    // the RoadManager library or the asset is unavailable in this environment.
    #[test]
    fn classifies_real_map_signals() {
        use esmini_rm::OdrMap;
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../external/esmini/resources/xodr/multi_intersections.xodr"
        );
        let Ok(map) = OdrMap::load(path) else {
            return;
        };
        let sigs = map.signals();
        assert!(!sigs.is_empty(), "map should expose signals");
        let lights = sigs
            .iter()
            .filter(|s| classify(&s.sign_type, &s.subtype).category == "traffic_light")
            .count();
        assert!(lights > 0, "expected at least one classified traffic light");
    }
}
