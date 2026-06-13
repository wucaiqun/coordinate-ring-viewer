use std::f64::consts::PI;

use egui::{Color32, Pos2, Rect, Response, RichText, Sense, Ui, Vec2};
use egui::epaint::Vertex;

/// Approximate meters per degree of latitude (WGS84 mean).
const METERS_PER_DEG_LAT: f64 = 111_320.0;
/// Minimum orbit distance — lower value allows stronger zoom-in.
const MIN_CAMERA_DISTANCE: f32 = 0.002;
const MAX_CAMERA_DISTANCE: f32 = 50.0;
const GRID_STEPS: i32 = 5;
const POINT_HALF_SIZE: f32 = 3.5;

const VIEW_CAPTION_SIZE: f32 = 14.0;
const VIEW_HINT_SIZE: f32 = 12.0;
const TEXT_PRIMARY: Color32 = Color32::from_rgb(235, 235, 235);
const TEXT_MUTED: Color32 = Color32::from_gray(150);
use crate::parser::{ParseResult3D, Point3D};
use crate::viewer2d::ring_color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickedPoint3D {
    pub ring_idx: usize,
    pub point_idx: usize,
}

#[derive(Default)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub pan_x: f64,
    pub pan_y: f64,
}

impl OrbitCamera {
    pub fn reset(&mut self) {
        *self = Self {
            yaw: 0.6,
            pitch: 0.35,
            distance: 3.0,
            pan_x: 0.0,
            pan_y: 0.0,
        };
    }
}

#[derive(Default)]
pub struct LocalScene3D {
    pub rings: Vec<Vec<(f64, f64, f64)>>,
    pub extent: f64,
    pub alt_min: f64,
    pub alt_max: f64,
}

pub fn build_local_scene(data: &ParseResult3D) -> LocalScene3D {
    let point_count: usize = data.rings.iter().map(|r| r.points.len()).sum();
    if point_count == 0 {
        return LocalScene3D::default();
    }

    let mut sum_lon = 0.0;
    let mut sum_lat = 0.0;
    for ring in &data.rings {
        for p in &ring.points {
            sum_lon += p.lon;
            sum_lat += p.lat;
        }
    }
    let n = point_count as f64;
    let center_lon = sum_lon / n;
    let center_lat = sum_lat / n;
    let meters_per_deg_lon = METERS_PER_DEG_LAT * center_lat.to_radians().cos().max(0.01);

    let to_local = |p: Point3D| -> (f64, f64, f64) {
        let x = (p.lon - center_lon) * meters_per_deg_lon;
        let z = (center_lat - p.lat) * METERS_PER_DEG_LAT;
        let y = p.alt;
        (x, y, z)
    };

    let mut extent = 0.1_f64;
    let mut alt_min = f64::MAX;
    let mut alt_max = f64::MIN;
    let rings = data
        .rings
        .iter()
        .map(|ring| {
            ring.points
                .iter()
                .copied()
                .map(|p| {
                    let local = to_local(p);
                    extent = extent
                        .max(local.0.abs())
                        .max(local.1.abs())
                        .max(local.2.abs());
                    alt_min = alt_min.min(local.1);
                    alt_max = alt_max.max(local.1);
                    local
                })
                .collect()
        })
        .collect();

    if alt_min > alt_max {
        alt_min = 0.0;
        alt_max = 1.0;
    }

    LocalScene3D {
        rings,
        extent,
        alt_min,
        alt_max,
    }
}

pub fn show_3d_view(
    ui: &mut Ui,
    data: &ParseResult3D,
    scene: &LocalScene3D,
    camera: &mut OrbitCamera,
    selected: Option<PickedPoint3D>,
    picked_out: &mut Option<PickedPoint3D>,
    live_render: bool,
) -> Response {
    let point_count: usize = scene.rings.iter().map(|r| r.len()).sum();

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "3D view - {} ring(s), {} point(s)",
                    scene.rings.len(),
                    point_count
                ))
                .size(VIEW_CAPTION_SIZE)
                .color(TEXT_PRIMARY),
            );
            if ui.button("Reset view").clicked() {
                camera.reset();
                ui.ctx().request_repaint();
            }
        });
        ui.label(
            RichText::new(
                "Left-drag: pan | Right-drag: rotate | Scroll at cursor: zoom | Double-click: reset",
            )
            .size(VIEW_HINT_SIZE)
            .color(TEXT_MUTED),
        );
        ui.add_space(4.0);

        let size = Vec2::new(ui.available_width(), ui.available_height());
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let scale = fit_scale(scene.extent, rect);
        handle_camera_input(&response, camera, rect, scale);

        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                *picked_out = find_nearest_projected_point(scene, rect, camera, pointer);
            }
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            if live_render {
                draw_3d_scene(&painter, rect, data, scene, camera, selected);
            } else {
                draw_3d_paused_overlay(&painter, rect);
            }
        }

        response
    })
    .inner
}

fn handle_camera_input(response: &Response, camera: &mut OrbitCamera, rect: Rect, scale: f64) {
    let mut changed = false;
    let zoom = zoom_scale(scale, camera.distance);

    if response.hovered() {
        let scroll = response.ctx.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let old_zoom = zoom;
            camera.distance *= 1.0 - scroll * 0.001;
            camera.distance = camera.distance.clamp(MIN_CAMERA_DISTANCE, MAX_CAMERA_DISTANCE);
            let new_zoom = zoom_scale(scale, camera.distance);

            if let Some(cursor) = response.hover_pos() {
                let cx = f64::from(cursor.x - rect.center().x);
                let cy = f64::from(cursor.y - rect.center().y);
                let inv_old = 1.0 / f64::from(old_zoom);
                let inv_new = 1.0 / f64::from(new_zoom);
                camera.pan_x += cx * (inv_new - inv_old);
                camera.pan_y -= cy * (inv_new - inv_old);
            }
            changed = true;
        }
    }

    if response.double_clicked() {
        camera.reset();
        changed = true;
    }

    if response.dragged() {
        let delta = response.drag_delta();
        let secondary = response.ctx.input(|i| i.pointer.secondary_down());
        if secondary {
            camera.yaw += delta.x * 0.005;
            camera.pitch =
                (camera.pitch + delta.y * 0.005).clamp(-PI as f32 * 0.49, PI as f32 * 0.49);
        } else {
            camera.pan_x += f64::from(delta.x) / f64::from(zoom);
            camera.pan_y -= f64::from(delta.y) / f64::from(zoom);
        }
        changed = true;
    }

    if changed {
        response.ctx.request_repaint();
    }
}

fn zoom_scale(scale: f64, distance: f32) -> f32 {
    (scale / f64::from(distance.max(MIN_CAMERA_DISTANCE))) as f32
}

struct Projector {
    center_x: f32,
    center_y: f32,
    cos_y: f32,
    sin_y: f32,
    cos_p: f32,
    sin_p: f32,
    zoom_scale: f32,
    pan_x: f32,
    pan_y: f32,
}

impl Projector {
    fn new(center: Pos2, scale: f64, camera: &OrbitCamera) -> Self {
        Self {
            center_x: center.x,
            center_y: center.y,
            cos_y: camera.yaw.cos(),
            sin_y: camera.yaw.sin(),
            cos_p: camera.pitch.cos(),
            sin_p: camera.pitch.sin(),
            zoom_scale: (scale / f64::from(camera.distance.max(MIN_CAMERA_DISTANCE))) as f32,
            pan_x: camera.pan_x as f32,
            pan_y: camera.pan_y as f32,
        }
    }

    #[inline]
    fn project(&self, x: f64, y: f64, z: f64) -> Pos2 {
        let x = x as f32;
        let y = y as f32;
        let z = z as f32;
        let x1 = x * self.cos_y - z * self.sin_y;
        let z1 = x * self.sin_y + z * self.cos_y;
        let ry = y * self.cos_p - z1 * self.sin_p;
        Pos2::new(
            self.center_x + (x1 + self.pan_x) * self.zoom_scale,
            self.center_y - (ry + self.pan_y) * self.zoom_scale,
        )
    }
}

fn draw_3d_paused_overlay(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, 8.0, Color32::from_rgb(15, 23, 42));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Editing coordinates…\nClick Draw to refresh the 3D view",
        egui::FontId::proportional(13.0),
        TEXT_MUTED,
    );
}

fn draw_3d_scene(
    painter: &egui::Painter,
    rect: Rect,
    data: &ParseResult3D,
    scene: &LocalScene3D,
    camera: &OrbitCamera,
    selected: Option<PickedPoint3D>,
) {
    painter.rect_filled(rect, 8.0, Color32::from_rgb(15, 23, 42));

    if scene.rings.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No 3D data",
            egui::FontId::proportional(14.0),
            TEXT_MUTED,
        );
        return;
    }

    let scale = fit_scale(scene.extent, rect);
    let projector = Projector::new(rect.center(), scale, camera);

    draw_grid(painter, &projector, scene.extent);

    for (idx, ring) in scene.rings.iter().enumerate() {
        let color = ring_color(idx);
        let screen: Vec<Pos2> = ring
            .iter()
            .map(|&(x, y, z)| projector.project(x, y, z))
            .collect();

        for window in screen.windows(2) {
            painter.line_segment([window[0], window[1]], egui::Stroke::new(2.0, color));
        }
        if screen.len() >= 3 {
            painter.line_segment(
                [screen[screen.len() - 1], screen[0]],
                egui::Stroke::new(2.0, color),
            );
        }
    }

    let mut selected_screen: Option<Pos2> = None;
    let mut points_mesh = egui::Mesh::default();
    for (idx, ring) in scene.rings.iter().enumerate() {
        for (point_idx, &(x, y, z)) in ring.iter().enumerate() {
            let is_selected = selected
                .map(|picked| picked.ring_idx == idx && picked.point_idx == point_idx)
                .unwrap_or(false);
            let pos = projector.project(x, y, z);
            if is_selected {
                selected_screen = Some(pos);
                continue;
            }
            let color = altitude_color(y, scene.alt_min, scene.alt_max);
            append_point_quad(&mut points_mesh, pos, POINT_HALF_SIZE, color);
        }
    }
    if !points_mesh.is_empty() {
        painter.add(egui::Shape::mesh(points_mesh));
    }

    if let Some(pos) = selected_screen {
        painter.circle_filled(pos, 6.0, Color32::WHITE);
        painter.circle_stroke(pos, 8.5, egui::Stroke::new(1.5, Color32::WHITE));
    }

    if let Some(picked) = selected {
        if let (Some(ring), Some(local)) = (
            data.rings.get(picked.ring_idx),
            scene
                .rings
                .get(picked.ring_idx)
                .and_then(|r| r.get(picked.point_idx)),
        ) {
            if let Some(point) = ring.points.get(picked.point_idx) {
                let pos = projector.project(local.0, local.1, local.2);
                draw_coord_label(
                    painter,
                    pos,
                    &format_coord_3d(point.lon, point.lat, point.alt),
                );
            }
        }
    }

    draw_axes(painter, &projector, scene.extent);
}

/// Map altitude (local Y, meters) to a blue → green → red gradient.
fn altitude_color(alt: f64, alt_min: f64, alt_max: f64) -> Color32 {
    let span = (alt_max - alt_min).max(1.0);
    let t = ((alt - alt_min) / span).clamp(0.0, 1.0) as f32;
    let (r, g, b) = if t < 0.5 {
        let u = t * 2.0;
        (
            lerp_channel(59.0, 34.0, u),
            lerp_channel(130.0, 197.0, u),
            lerp_channel(246.0, 94.0, u),
        )
    } else {
        let u = (t - 0.5) * 2.0;
        (
            lerp_channel(34.0, 239.0, u),
            lerp_channel(197.0, 68.0, u),
            lerp_channel(94.0, 68.0, u),
        )
    };
    Color32::from_rgb(r as u8, g as u8, b as u8)
}

fn lerp_channel(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// One screen-space quad (two triangles) for a point.
fn append_point_quad(mesh: &mut egui::Mesh, center: Pos2, half: f32, color: Color32) {
    let base = mesh.vertices.len() as u32;
    let corners = [
        Pos2::new(center.x - half, center.y - half),
        Pos2::new(center.x + half, center.y - half),
        Pos2::new(center.x + half, center.y + half),
        Pos2::new(center.x - half, center.y + half),
    ];
    for pos in corners {
        mesh.vertices.push(Vertex {
            pos,
            uv: Pos2::ZERO,
            color,
        });
    }
    mesh.add_triangle(base, base + 1, base + 2);
    mesh.add_triangle(base, base + 2, base + 3);
}

fn find_nearest_projected_point(
    scene: &LocalScene3D,
    rect: Rect,
    camera: &OrbitCamera,
    cursor: Pos2,
) -> Option<PickedPoint3D> {
    if scene.rings.is_empty() {
        return None;
    }

    let projector = Projector::new(rect.center(), fit_scale(scene.extent, rect), camera);
    let mut best: Option<(PickedPoint3D, f32)> = None;

    for (ring_idx, ring) in scene.rings.iter().enumerate() {
        for (point_idx, &(x, y, z)) in ring.iter().enumerate() {
            let pos = projector.project(x, y, z);
            let dist = pos.distance(cursor);
            if best.is_none_or(|(_, best_dist)| dist < best_dist) {
                best = Some((PickedPoint3D { ring_idx, point_idx }, dist));
            }
        }
    }

    best.map(|(picked, _)| picked)
}

fn fit_scale(extent: f64, rect: Rect) -> f64 {
    let min_dim = f64::from(rect.width().min(rect.height())) * 0.35;
    min_dim / extent.max(0.1)
}

fn draw_grid(painter: &egui::Painter, projector: &Projector, extent: f64) {
    let grid_color = Color32::from_rgb(30, 41, 59);
    for i in -GRID_STEPS..=GRID_STEPS {
        let t = i as f64 / f64::from(GRID_STEPS) * extent;
        let a = projector.project(t, 0.0, -extent);
        let b = projector.project(t, 0.0, extent);
        painter.line_segment([a, b], egui::Stroke::new(1.0, grid_color));

        let c = projector.project(-extent, 0.0, t);
        let d = projector.project(extent, 0.0, t);
        painter.line_segment([c, d], egui::Stroke::new(1.0, grid_color));
    }
}

fn draw_axes(painter: &egui::Painter, projector: &Projector, extent: f64) {
    let origin = projector.project(0.0, 0.0, 0.0);
    let axis_len = extent * 0.4;
    let axes = [
        (axis_len, 0.0, 0.0, Color32::from_rgb(239, 68, 68), "X"),
        (0.0, axis_len, 0.0, Color32::from_rgb(34, 197, 94), "Y"),
        (0.0, 0.0, axis_len, Color32::from_rgb(59, 130, 246), "Z"),
    ];
    for (x, y, z, color, label) in axes {
        let end = projector.project(x, y, z);
        painter.line_segment([origin, end], egui::Stroke::new(2.5, color));
        let (label_pos, align) = axis_label_layout(origin, end);
        painter.text(label_pos, align, label, egui::FontId::proportional(12.0), color);
    }
}

/// Place axis labels just past each tip, extending away from the origin in screen space.
fn axis_label_layout(origin: Pos2, end: Pos2) -> (Pos2, egui::Align2) {
    let dir = end - origin;
    let len = dir.length();
    if len < 0.5 {
        return (end, egui::Align2::CENTER_CENTER);
    }

    let norm = dir / len;
    let label_pos = end + norm * 14.0;
    let align = if norm.x.abs() >= norm.y.abs() {
        if norm.x >= 0.0 {
            egui::Align2::LEFT_CENTER
        } else {
            egui::Align2::RIGHT_CENTER
        }
    } else if norm.y >= 0.0 {
        egui::Align2::CENTER_TOP
    } else {
        egui::Align2::CENTER_BOTTOM
    };

    (label_pos, align)
}

fn format_coord_3d(lon: f64, lat: f64, alt: f64) -> String {
    format!("{lon:.6} {lat:.6} {alt:.1}m")
}

fn draw_coord_label(painter: &egui::Painter, anchor: Pos2, text: &str) {
    let font_id = egui::FontId::proportional(11.0);
    let galley = painter.layout_no_wrap(text.to_string(), font_id, Color32::WHITE);
    let padding = egui::vec2(5.0, 3.0);
    let label_pos = anchor + egui::vec2(10.0, -8.0);
    let rect = egui::Align2::LEFT_BOTTOM.anchor_size(label_pos, galley.size() + padding * 2.0);
    painter.rect_filled(rect, 4.0, Color32::from_rgba_unmultiplied(15, 23, 42, 230));
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, Color32::from_rgb(100, 116, 139)),
        egui::epaint::StrokeKind::Outside,
    );
    painter.galley(rect.min + padding, galley, Color32::WHITE);
}
