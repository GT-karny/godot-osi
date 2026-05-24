//! `OsiTrafficLightVisualizer`: an optional `Node3D` that renders OpenDRIVE
//! traffic lights as 3D models and lets you switch each lamp on/off.
//!
//! Approach (after esmini's `trafficlightmodel.cpp`): each dynamic
//! `category == "traffic_light"` signal becomes a dark housing box plus `N`
//! front "lamp" quads (`N == nr_lamps`). The catalogue icon PNG
//! (`addons/godot_osi/icons/signals/<icon>.png`) is a vertical stack of the `N`
//! lamps, so each lamp quad samples one horizontal band of it via UV slicing
//! (lamp `i` -> `v = [i/N, (i+1)/N]`). A lit lamp shows its textured, unshaded,
//! emissive quad; an unlit lamp hides the quad so only the dark housing shows.
//!
//! Roads/signals are static, so this builds once on demand; the lamp *state* is
//! then driven through the [`OsiTrafficLightVisualizer::set_lamp`] /
//! [`OsiTrafficLightVisualizer::set_color_state`] API (manual for now — the OSI
//! converter does not yet produce traffic-light phases).
//!
//! Orientation note: a signal faces its `heading` direction, which maps to Godot
//! **+X** under the default [`AxisMapping`] (OSI forward `+x` -> Godot `+x`).
//! Lamp quads therefore face local `+X`; height is `+Y`, lateral is `Z`.

use std::collections::HashMap;

use godot::classes::base_material_3d::{CullMode, ShadingMode, TextureParam, Transparency};
use godot::classes::mesh::PrimitiveType;
use godot::classes::{
    ArrayMesh, BoxMesh, Image, ImageTexture, Material, MeshInstance3D, Node3D, ProjectSettings,
    StandardMaterial3D, SurfaceTool, Texture2D,
};
use godot::prelude::*;
use osi_types::osi3;

use super::network::{to_godot, OsiRoadNetwork};
use super::signal_catalog;
use crate::converter::coords::{osi_orientation_to_basis, AxisMapping};

/// A built traffic-light head, kept so the API can toggle individual lamps.
struct Head {
    /// One quad `MeshInstance3D` per lamp, index 0 = top lamp.
    lamps: Vec<Gd<MeshInstance3D>>,
    /// Lamp colour names parallel to `lamps` (see [`lamp_colors_for`]).
    colors: Vec<String>,
}

#[derive(GodotClass)]
#[class(base=Node3D, init)]
pub struct OsiTrafficLightVisualizer {
    /// Uniform scale forwarded to the coordinate mapping (meters by default).
    #[var]
    #[init(val = 1.0)]
    scale: real,
    /// Multiplier on the physical head size derived from the signal/catalog dims.
    #[var]
    #[init(val = 1.0)]
    size_scale: real,
    /// Emission energy of a lit lamp (HDR glow strength).
    #[var]
    #[init(val = 1.6)]
    emission_energy: real,
    /// Flip every head 180° about up. esmini's signal heading points along the
    /// sign's back normal, so the lamp face (local +X) ends up away from the
    /// viewer without this; default `true` turns the lamps toward traffic. The
    /// per-signal `orientation` flag is XORed with this. Set false if a given
    /// map's signals end up reversed.
    #[var]
    #[init(val = true)]
    flip_facing: bool,
    /// Lay multi-lamp **vehicle** heads out horizontally (a row of lamps) instead
    /// of the default vertical stack — e.g. the common Japanese arrangement.
    /// Pedestrian/bicycle heads stay vertical regardless.
    #[var]
    #[init(val = false)]
    horizontal: bool,

    /// Heads keyed by OpenDRIVE `global_id` (`GetGlobalId`, unique per signal).
    /// Not a `#[var]`: `Gd` handles cannot be exported to the editor.
    heads: HashMap<i64, Head>,
    /// Maps a signal's per-road `id` (`GetId`) to its `global_id`. esmini's OSI
    /// `source_reference` carries `GetId`, so this routes an incoming OSI
    /// traffic light to its head (see [`Self::set_color_state_by_signal_id`]).
    sig_index: HashMap<i64, i64>,
    /// Icon stem -> decoded texture, reused across rebuilds to avoid re-decoding.
    tex_cache: HashMap<String, Gd<ImageTexture>>,
    /// Parent of all head sub-trees; freed and rebuilt on each `build_from`.
    root: Option<Gd<Node3D>>,
    /// Demo phase counter for [`Self::cycle_demo`].
    demo_phase: i64,
    base: Base<Node3D>,
}

#[godot_api]
impl OsiTrafficLightVisualizer {
    /// (Re)build all dynamic traffic-light heads from `network`. Frees any
    /// previous render. Lamps start unlit; drive them via the state API below.
    #[func]
    fn build_from(&mut self, network: Gd<OsiRoadNetwork>) {
        if let Some(mut n) = self.root.take() {
            n.queue_free();
        }
        self.heads.clear();
        self.sig_index.clear();
        self.demo_phase = 0;

        let mapping = AxisMapping {
            scale: self.scale,
            ..Default::default()
        };

        // Collect plain signal data while the network is borrowed (no `Gd` yet).
        let collected: Vec<PlannedHead> = {
            let net = network.bind();
            let Some(map) = net.map() else {
                godot_warn!("[OsiTrafficLightVisualizer] network not loaded");
                return;
            };
            map.signals()
                .into_iter()
                .filter_map(|s| {
                    let c = signal_catalog::classify(&s.sign_type, &s.subtype);
                    if !(s.dynamic && c.category == "traffic_light") {
                        return None;
                    }
                    let nr_lamps = (c.nr_lamps.max(1)) as usize;
                    // `s.z` is the road-surface elevation; `z_offset` is the mast
                    // mounting height above it (esmini does not fold it in). Add
                    // it so heads sit on the pole — this also separates signals
                    // sharing one mast at different heights.
                    let pos = to_godot([s.x, s.y, s.z + s.z_offset], &mapping);
                    // `heading` is the road reference heading; `h_offset` is the
                    // signal's own rotation about up. The facing is their sum.
                    let basis = osi_orientation_to_basis(
                        &osi3::Orientation3d {
                            yaw: Some(s.heading + s.h_offset),
                            pitch: Some(s.pitch),
                            roll: Some(s.roll),
                        },
                        &mapping,
                    );
                    Some(PlannedHead {
                        global_id: s.global_id as i64,
                        signal_id: s.id as i64,
                        pos,
                        basis,
                        orientation: s.orientation,
                        width: s.width,
                        height: s.height,
                        depth: s.depth,
                        icon: c.icon.clone(),
                        nr_lamps,
                        subcategory: c.subcategory.clone(),
                        color: c.color.clone(),
                    })
                })
                .collect()
        };

        let size_scale = self.size_scale;
        let emission = self.emission_energy;
        let flip_facing = self.flip_facing;
        let horizontal = self.horizontal;

        let mut root = Node3D::new_alloc();
        root.set_name("TrafficLights");
        let mut heads_map: HashMap<i64, Head> = HashMap::new();
        let mut sig_index: HashMap<i64, i64> = HashMap::new();

        for ph in &collected {
            let tex = self.texture_for(&ph.icon);
            let (head_node, head) = build_head(ph, tex, size_scale, emission, flip_facing, horizontal);
            root.add_child(&head_node);
            heads_map.insert(ph.global_id, head);
            if let Some(prev) = sig_index.insert(ph.signal_id, ph.global_id) {
                godot_warn!(
                    "[OsiTrafficLightVisualizer] duplicate signal id {} (global {} and {}); \
                     OSI routing keeps the latter",
                    ph.signal_id,
                    prev,
                    ph.global_id
                );
            }
        }

        self.base_mut().add_child(&root);
        self.root = Some(root);
        self.heads = heads_map;
        self.sig_index = sig_index;
    }

    /// Turn a single lamp (by `lamp_index`, 0 = top) on or off.
    #[func]
    fn set_lamp(&mut self, global_id: i64, lamp_index: i64, on: bool) {
        if let Some(head) = self.heads.get_mut(&global_id) {
            if let Some(lamp) = usize::try_from(lamp_index)
                .ok()
                .and_then(|i| head.lamps.get_mut(i))
            {
                lamp.set_visible(on);
            }
        }
    }

    /// Light only the lamp of the given `color` (`"red"`/`"yellow"`/`"green"`);
    /// `"off"`/`"none"` (or an absent colour) turns the head fully dark.
    #[func]
    fn set_color_state(&mut self, global_id: i64, color: GString) {
        let color = color.to_string();
        if let Some(head) = self.heads.get_mut(&global_id) {
            for lamp in head.lamps.iter_mut() {
                lamp.set_visible(false);
            }
            if let Some(i) = color_to_index(&head.colors, &color) {
                head.lamps[i].set_visible(true);
            }
        }
    }

    /// Turn every lamp of a head off.
    #[func]
    fn all_off(&mut self, global_id: i64) {
        if let Some(head) = self.heads.get_mut(&global_id) {
            for lamp in head.lamps.iter_mut() {
                lamp.set_visible(false);
            }
        }
    }

    /// Step a red -> yellow -> green demo across all heads (manual driver).
    #[func]
    fn cycle_demo(&mut self) {
        const PHASES: [&str; 3] = ["red", "yellow", "green"];
        let color = PHASES[(self.demo_phase.rem_euclid(3)) as usize];
        self.demo_phase = self.demo_phase.wrapping_add(1);
        let ids: Vec<i64> = self.heads.keys().copied().collect();
        for id in ids {
            self.set_color_state(id, GString::from(color));
        }
    }

    /// Number of traffic-light heads currently built.
    #[func]
    fn head_count(&self) -> i64 {
        self.heads.len() as i64
    }

    /// All built heads' `global_id`s, sorted ascending (deterministic order so
    /// consumers can map e.g. the i-th OSI light to the i-th head).
    #[func]
    fn global_ids(&self) -> PackedInt64Array {
        let mut ids: Vec<i64> = self.heads.keys().copied().collect();
        ids.sort_unstable();
        ids.into_iter().collect()
    }

    /// Route an OSI traffic-light update to its head by the OpenDRIVE signal id
    /// (`GetId`) that esmini puts in `source_reference` (`"traffic_light_id:<N>"`).
    /// Lights only `color` (`"red"`/`"yellow"`/`"green"`; anything else turns the
    /// head off). Returns `true` if a head with that signal id exists.
    ///
    /// This is the deterministic, position-free binding between an OSI
    /// `TrafficLight` and a rendered head.
    #[func]
    fn set_color_state_by_signal_id(&mut self, signal_id: i64, color: GString) -> bool {
        let Some(&global_id) = self.sig_index.get(&signal_id) else {
            return false;
        };
        self.set_color_state(global_id, color);
        true
    }

    /// Number of lamps in a head (0 if unknown).
    #[func]
    fn lamp_count(&self, global_id: i64) -> i64 {
        self.heads
            .get(&global_id)
            .map(|h| h.lamps.len() as i64)
            .unwrap_or(0)
    }

    /// Whether a given lamp is currently lit (test/assert hook).
    #[func]
    fn is_lamp_on(&self, global_id: i64, lamp_index: i64) -> bool {
        self.heads
            .get(&global_id)
            .and_then(|h| usize::try_from(lamp_index).ok().and_then(|i| h.lamps.get(i)))
            .map(|l| l.is_visible())
            .unwrap_or(false)
    }

    /// Load (and cache) the catalogue icon texture for `icon` (a file stem like
    /// `"odr_1000001_none"`), or `None` for an empty stem / load failure.
    ///
    /// The PNGs ship without `.import` files, so `ResourceLoader` can't see them
    /// in a headless run; we resolve the `res://` path to an OS path and decode
    /// the file directly, mirroring [`OsiRoadNetwork::load`].
    fn texture_for(&mut self, icon: &str) -> Option<Gd<ImageTexture>> {
        if icon.is_empty() {
            return None;
        }
        if let Some(t) = self.tex_cache.get(icon) {
            return Some(t.clone());
        }
        let res = format!("res://addons/godot_osi/icons/signals/{icon}.png");
        let os_path = ProjectSettings::singleton()
            .globalize_path(&GString::from(res.as_str()))
            .to_string();
        let Some(img) = Image::load_from_file(&GString::from(os_path.as_str())) else {
            godot_warn!("[OsiTrafficLightVisualizer] could not load icon: {os_path}");
            return None;
        };
        let Some(tex) = ImageTexture::create_from_image(&img) else {
            godot_warn!("[OsiTrafficLightVisualizer] could not build texture: {icon}");
            return None;
        };
        self.tex_cache.insert(icon.to_string(), tex.clone());
        Some(tex)
    }
}

/// Plain (no-`Gd`) signal data gathered while the network is borrowed.
struct PlannedHead {
    global_id: i64,
    /// Per-road signal id (`GetId`); the key esmini's OSI `source_reference` uses.
    signal_id: i64,
    pos: Vector3,
    basis: Basis,
    orientation: i32,
    width: f64,
    height: f64,
    depth: f64,
    icon: String,
    nr_lamps: usize,
    subcategory: String,
    color: String,
}

/// Build one traffic-light head (housing box + lamp quads) and return its root
/// `Node3D` plus the [`Head`] handle used for state control. All lamps start
/// hidden (unlit).
fn build_head(
    ph: &PlannedHead,
    tex: Option<Gd<ImageTexture>>,
    size_scale: real,
    emission: real,
    flip_facing: bool,
    horizontal: bool,
) -> (Gd<Node3D>, Head) {
    let s = size_scale as f64;
    let n = ph.nr_lamps;
    // Horizontal layout is for multi-lamp vehicle heads only; pedestrian and
    // bicycle signals stay vertical (matching real intersections).
    let lay_horizontal =
        horizontal && n >= 2 && matches!(ph.subcategory.as_str(), "vehicle" | "vehicle_arrow");

    // Forward = local +X, up = +Y, lateral = Z (see module docs).
    let box_d = (if ph.depth > 0.02 { ph.depth } else { 0.10 }) * s;
    // Length along the lamp axis (the vertical board height by default).
    let span = (if ph.height > 0.05 { ph.height } else { 0.40 * n as f64 }) * s;
    let board_w = (if ph.width > 0.05 { ph.width } else { 0.40 }) * s;
    let pitch = span / n as f64; // per-lamp spacing along the lamp axis
    let front_x = box_d * 0.5 + 0.005;

    // Housing extents and per-lamp quad half-sizes per layout.
    //   Vertical:   lamps along Y; housing span(Y) x board_w(Z).
    //   Horizontal: lamps along Z in square `pitch` cells; row length = span.
    let (box_size, lamp_half_y, lamp_half_z) = if lay_horizontal {
        let half = (pitch * 0.90) * 0.5;
        (
            Vector3::new(box_d as real, pitch as real, span as real),
            half,
            half,
        )
    } else {
        (
            Vector3::new(box_d as real, span as real, board_w as real),
            (pitch * 0.90) * 0.5,
            (board_w * 0.90) * 0.5,
        )
    };

    // Per-lamp colour order. Single-aspect heads use the catalogue colour.
    let colors: Vec<String> = if ph.nr_lamps == 1 {
        let c = if matches!(ph.color.as_str(), "red" | "yellow" | "green") {
            ph.color.as_str()
        } else {
            "single"
        };
        vec![c.to_string()]
    } else {
        lamp_colors_for(&ph.subcategory, ph.nr_lamps)
            .into_iter()
            .map(str::to_string)
            .collect()
    };

    let mut head_node = Node3D::new_alloc();
    head_node.set_name(&format!("tl_{}", ph.global_id));

    // Position + orientation; optionally flip 180° about up to face traffic.
    let flip = flip_facing ^ (ph.orientation == 1);
    let mut basis = ph.basis;
    if flip {
        basis = basis * Basis::from_axis_angle(Vector3::new(0.0, 1.0, 0.0), std::f32::consts::PI as real);
    }
    head_node.set_transform(Transform3D::new(basis, ph.pos));

    // Dark housing box (closed; reads like esmini's open box from the front).
    let mut housing_mesh = BoxMesh::new_gd();
    housing_mesh.set_size(box_size);
    let mut hmat = StandardMaterial3D::new_gd();
    hmat.set_albedo(Color::from_rgba(0.07, 0.07, 0.08, 1.0));
    let hmat: Gd<Material> = hmat.upcast();
    housing_mesh.set_material(&hmat);
    let mut housing = MeshInstance3D::new_alloc();
    housing.set_name("housing");
    housing.set_mesh(&housing_mesh);
    head_node.add_child(&housing);

    // Lamp quads, index 0 first (top for vertical, +Z end for horizontal). Skip
    // if no artwork is available.
    let mut lamps: Vec<Gd<MeshInstance3D>> = Vec::new();
    if let Some(tex) = tex {
        let lamp_mat = lamp_material(&tex, emission);
        for i in 0..n {
            // Offset of lamp i from centre along its layout axis (index 0 first).
            let off = span * 0.5 - pitch * (i as f64 + 0.5);
            // Vertical: index 0 (red) on top (+Y). Horizontal: index 0 (red) on
            // the viewer's right. The viewer reads the lamp face (local +X)
            // looking along -X with up +Y, so their right is -Z (Godot is
            // right-handed); hence red -> -Z. This holds regardless of flip_facing
            // (that only rotates the whole head; the viewer is always on the lit
            // side), so red is always on the right.
            let (y_c, z_c) = if lay_horizontal { (0.0, -off) } else { (off, 0.0) };
            let uv = uv_slice(i, n);
            let Some(mesh) = build_lamp_quad_mesh(
                front_x as real,
                y_c as real,
                z_c as real,
                lamp_half_y as real,
                lamp_half_z as real,
                uv,
            ) else {
                continue;
            };
            let mut lamp = MeshInstance3D::new_alloc();
            lamp.set_name(&format!("lamp_{i}"));
            lamp.set_mesh(&mesh);
            lamp.set_material_override(&lamp_mat);
            lamp.set_visible(false); // unlit until the state API turns it on
            head_node.add_child(&lamp);
            lamps.push(lamp);
        }
    } else if !ph.icon.is_empty() {
        godot_warn!(
            "[OsiTrafficLightVisualizer] head {} has icon '{}' but no texture; housing only",
            ph.global_id,
            ph.icon
        );
    }

    // Keep `colors` aligned with the lamps we actually built.
    let mut colors = colors;
    colors.truncate(lamps.len());

    (head_node, Head { lamps, colors })
}

/// Shared unshaded, emissive, alpha-blended material for the lamp quads.
fn lamp_material(tex: &Gd<ImageTexture>, emission: real) -> Gd<Material> {
    let mut mat = StandardMaterial3D::new_gd();
    mat.set_shading_mode(ShadingMode::UNSHADED);
    mat.set_cull_mode(CullMode::DISABLED);
    mat.set_transparency(Transparency::ALPHA);
    let tex2d: Gd<Texture2D> = tex.clone().upcast();
    mat.set_texture(TextureParam::ALBEDO, &tex2d);
    mat.set_texture(TextureParam::EMISSION, &tex2d);
    mat.set_emission(Color::from_rgba(1.0, 1.0, 1.0, 1.0));
    mat.set_emission_energy_multiplier(emission as f32);
    mat.upcast()
}

/// A single lamp quad facing local `+X`, centred at (`y_c` up, `z_c` lateral)
/// with half-extents `half_y` (up) and `half_z` (lateral). UVs come from
/// [`uv_slice`]. Culling is disabled by the material, so winding is irrelevant;
/// the normal is `+X` (unused while unshaded).
fn build_lamp_quad_mesh(
    front_x: real,
    y_c: real,
    z_c: real,
    half_y: real,
    half_z: real,
    uv: (f32, f32, f32, f32),
) -> Option<Gd<ArrayMesh>> {
    let (u0, v0, u1, v1) = uv; // v0 = top band edge, v1 = bottom band edge
    let n = Vector3::new(1.0, 0.0, 0.0);
    // Corners: bottom-left, bottom-right, top-right, top-left (lateral on Z).
    let bl = Vector3::new(front_x, y_c - half_y, z_c - half_z);
    let br = Vector3::new(front_x, y_c - half_y, z_c + half_z);
    let tr = Vector3::new(front_x, y_c + half_y, z_c + half_z);
    let tl = Vector3::new(front_x, y_c + half_y, z_c - half_z);
    let uv_bl = Vector2::new(u0 as real, v1 as real);
    let uv_br = Vector2::new(u1 as real, v1 as real);
    let uv_tr = Vector2::new(u1 as real, v0 as real);
    let uv_tl = Vector2::new(u0 as real, v0 as real);

    let mut st = SurfaceTool::new_gd();
    st.begin(PrimitiveType::TRIANGLES);
    for (p, t) in [
        (bl, uv_bl),
        (br, uv_br),
        (tr, uv_tr),
        (bl, uv_bl),
        (tr, uv_tr),
        (tl, uv_tl),
    ] {
        st.set_normal(n);
        st.set_uv(t);
        st.add_vertex(p);
    }
    st.commit()
}

/// UV rectangle `(u_min, v_min, u_max, v_max)` for lamp `i` of `n` in a vertical
/// texture stack. The image's top row (`v = 0`) is lamp 0 (the top lamp).
pub(crate) fn uv_slice(i: usize, n: usize) -> (f32, f32, f32, f32) {
    let n = n.max(1);
    let i = i.min(n - 1);
    (0.0, i as f32 / n as f32, 1.0, (i + 1) as f32 / n as f32)
}

/// Lamp colours top-to-bottom for a head, given its catalogue `subcategory` and
/// lamp count. The catalogue stores `color == "multi"` for multi-aspect heads
/// without a per-lamp breakdown, so this encodes the standard physical order.
pub(crate) fn lamp_colors_for(subcategory: &str, nr_lamps: usize) -> Vec<&'static str> {
    match (subcategory, nr_lamps) {
        // Three-aspect (vehicle / vehicle_arrow): red, yellow, green top-down.
        (_, 3) => vec!["red", "yellow", "green"],
        // Two-aspect pedestrian/bicycle (and vehicle) heads: red over green.
        (_, 2) => vec!["red", "green"],
        (_, 1) => vec!["single"],
        _ => vec!["single"; nr_lamps.max(1)],
    }
}

/// Index of the lamp whose colour equals `want`, if any.
pub(crate) fn color_to_index(colors: &[String], want: &str) -> Option<usize> {
    colors.iter().position(|c| c == want)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_slice_three_aspect_bands() {
        assert_eq!(uv_slice(0, 3), (0.0, 0.0, 1.0, 1.0 / 3.0));
        let (_, v0, _, v1) = uv_slice(2, 3);
        assert!((v0 - 2.0 / 3.0).abs() < 1e-6);
        assert!((v1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn uv_slice_single_is_full() {
        assert_eq!(uv_slice(0, 1), (0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn uv_slice_clamps_out_of_range() {
        assert_eq!(uv_slice(5, 3), uv_slice(2, 3));
        assert_eq!(uv_slice(0, 0), (0.0, 0.0, 1.0, 1.0));
    }

    #[test]
    fn lamp_colors_three_aspect() {
        assert_eq!(lamp_colors_for("vehicle", 3), vec!["red", "yellow", "green"]);
    }

    #[test]
    fn lamp_colors_two_aspect() {
        assert_eq!(lamp_colors_for("pedestrian", 2), vec!["red", "green"]);
        assert_eq!(lamp_colors_for("bicycle", 2), vec!["red", "green"]);
    }

    #[test]
    fn lamp_colors_single() {
        assert_eq!(lamp_colors_for("vehicle_arrow", 1).len(), 1);
    }

    #[test]
    fn color_index_lookup() {
        let colors = vec!["red".to_string(), "yellow".to_string(), "green".to_string()];
        assert_eq!(color_to_index(&colors, "green"), Some(2));
        assert_eq!(color_to_index(&colors, "red"), Some(0));
        assert_eq!(color_to_index(&colors, "blue"), None);
    }


    // End-to-end guard mirroring signal_catalog::classifies_real_map_signals:
    // multi_intersections.xodr has dynamic OpenDRIVE-country lights, so the
    // build-side filter (`dynamic && traffic_light`) must select some. Skips
    // gracefully if the RoadManager library or the asset is unavailable.
    #[test]
    fn real_map_has_dynamic_traffic_lights() {
        use esmini_rm::OdrMap;
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../external/esmini/resources/xodr/multi_intersections.xodr"
        );
        let Ok(map) = OdrMap::load(path) else {
            return;
        };
        let heads = map
            .signals()
            .into_iter()
            .filter(|s| {
                let c = signal_catalog::classify(&s.sign_type, &s.subtype);
                s.dynamic && c.category == "traffic_light"
            })
            .count();
        assert!(heads > 0, "expected at least one dynamic traffic light");
    }
}
