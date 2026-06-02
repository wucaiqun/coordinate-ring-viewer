use std::f64::consts::PI;

use egui::{Color32, Pos2, Rect, Response, RichText, Sense, Ui, Vec2};

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
    pub pan_x: f32,
    pub pan_y: f32,
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

pub struct LocalScene3D {
    pub points: Vec<(f32, f32, f32)>,
    pub rings: Vec<Vec<(f32, f32, f32)>>,
}

pub fn build_local_scene(data: &ParseResult3D) -> LocalScene3D {
    let all: Vec<Point3D> = data.rings.iter().flat_map(|r| r.points.clone()).collect();
    if all.is_empty() {
        return LocalScene3D {
            points: vec![],
            rings: vec![],
        };
    }

    let center_lon = all.iter().map(|p| p.lon).sum::<f64>() / all.len() as f64;
    let center_lat = all.iter().map(|p| p.lat).sum::<f64>() / all.len() as f64;
    let cos_lat = center_lat.to_radians().cos().max(0.01);

    let to_local = |p: Point3D| -> (f32, f32, f32) {
        let x = ((p.lon - center_lon) * cos_lat) as f32;
        let z = ((p.lat - center_lat) * -1.0) as f32;
        let y = (p.alt / 1000.0) as f32;
        (x, y, z)
    };

    let rings = data
        .rings
        .iter()
        .map(|ring| ring.points.iter().copied().map(to_local).collect())
        .collect();

    LocalScene3D {
        points: all.iter().copied().map(to_local).collect(),
        rings,
    }
}

pub fn show_3d_view(
    ui: &mut Ui,
    data: &ParseResult3D,
    camera: &mut OrbitCamera,
    selected: Option<PickedPoint3D>,
    picked_out: &mut Option<PickedPoint3D>,
) -> Response {
    let scene = build_local_scene(data);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!(
                    "3D view - {} ring(s), {} point(s)",
                    data.rings.len(),
                    scene.points.len()
                ))
                .size(VIEW_CAPTION_SIZE)
                .color(TEXT_PRIMARY),
            );
            if ui.button("Reset view").clicked() {
                camera.reset();
            }
        });
        ui.label(
            RichText::new("Drag: rotate | Scroll: zoom | Shift+drag: pan")
                .size(VIEW_HINT_SIZE)
                .color(TEXT_MUTED),
        );
        ui.add_space(4.0);

        let size = Vec2::new(ui.available_width(), ui.available_height());
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        handle_camera_input(&response, camera);

        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                *picked_out = find_nearest_projected_point(data, rect, camera, pointer);
            }
        }

        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            draw_3d_scene(&painter, rect, &scene, camera, selected);
        }

        response
    })
    .inner
}

fn handle_camera_input(response: &Response, camera: &mut OrbitCamera) {
    if response.hovered() {
        camera.distance *= 1.0 - response.ctx.input(|i| i.raw_scroll_delta.y) * 0.001;
        camera.distance = camera.distance.clamp(0.5, 50.0);
    }

    if response.dragged() {
        let delta = response.drag_delta();
        let shift = response.ctx.input(|i| i.modifiers.shift);
        if shift {
            camera.pan_x += delta.x * 0.002;
            camera.pan_y += delta.y * 0.002;
        } else {
            camera.yaw += delta.x * 0.005;
            camera.pitch = (camera.pitch + delta.y * 0.005).clamp(-PI as f32 * 0.49, PI as f32 * 0.49);
        }
    }
}

fn draw_3d_scene(
    painter: &egui::Painter,
    rect: Rect,
    scene: &LocalScene3D,
    camera: &OrbitCamera,
    selected: Option<PickedPoint3D>,
) {
    painter.rect_filled(rect, 8.0, Color32::from_rgb(15, 23, 42));

    if scene.points.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No 3D data",
            egui::FontId::proportional(14.0),
            TEXT_MUTED,
        );
        return;
    }

    let scale = fit_scale(scene, rect);
    let project = |x: f32, y: f32, z: f32| -> Option<Pos2> {
        project_point(x, y, z, rect.center(), scale, camera)
    };

    draw_grid(painter, rect, scale, camera);

    for (idx, ring) in scene.rings.iter().enumerate() {
        let color = ring_color(idx);
        if ring.len() >= 3 {
            let mut projected = Vec::new();
            for &(x, y, z) in ring {
                if let Some(p) = project(x, y, z) {
                    projected.push(p);
                }
            }
            if projected.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    projected.clone(),
                    color.gamma_multiply(0.12),
                    egui::Stroke::NONE,
                ));
            }
        }

        for window in ring.windows(2) {
            if let (Some(a), Some(b)) = (project(window[0].0, window[0].1, window[0].2), project(window[1].0, window[1].1, window[1].2)) {
                painter.line_segment([a, b], egui::Stroke::new(2.0, color));
            }
        }
        if ring.len() >= 3 {
            let first = ring[0];
            let last = ring[ring.len() - 1];
            if let (Some(a), Some(b)) = (project(last.0, last.1, last.2), project(first.0, first.1, first.2)) {
                painter.line_segment([a, b], egui::Stroke::new(2.0, color));
            }
        }

        for (point_idx, &local) in ring.iter().enumerate() {
            if let Some(pos) = project(local.0, local.1, local.2) {
                let is_selected = selected
                    .map(|picked| picked.ring_idx == idx && picked.point_idx == point_idx)
                    .unwrap_or(false);
                let radius = if is_selected { 6.0 } else { 4.0 };
                let stroke_radius = if is_selected { 8.5 } else { 6.0 };
                let fill_color = if is_selected { Color32::WHITE } else { color };
                painter.circle_filled(pos, radius, fill_color);
                painter.circle_stroke(pos, stroke_radius, egui::Stroke::new(1.0, color.gamma_multiply(0.8)));
            }
        }
    }

    draw_axes(painter, rect.center(), scale, camera);
}

fn find_nearest_projected_point(
    data: &ParseResult3D,
    rect: Rect,
    camera: &OrbitCamera,
    cursor: Pos2,
) -> Option<PickedPoint3D> {
    let scene = build_local_scene(data);
    if scene.rings.is_empty() {
        return None;
    }

    let scale = fit_scale(&scene, rect);
    let center = rect.center();
    let mut best: Option<(PickedPoint3D, f32)> = None;

    for (ring_idx, ring) in scene.rings.iter().enumerate() {
        for (point_idx, &(x, y, z)) in ring.iter().enumerate() {
            if let Some(pos) = project_point(x, y, z, center, scale, camera) {
                let dist = pos.distance(cursor);
                match best {
                    Some((_, best_dist)) if dist >= best_dist => {}
                    _ => {
                        best = Some((PickedPoint3D { ring_idx, point_idx }, dist));
                    }
                }
            }
        }
    }

    best.map(|(picked, _)| picked)
}

fn fit_scale(scene: &LocalScene3D, rect: Rect) -> f32 {
    let mut max_extent = 0.1_f32;
    for &(x, y, z) in &scene.points {
        max_extent = max_extent.max(x.abs()).max(y.abs()).max(z.abs());
    }
    let min_dim = rect.width().min(rect.height()) * 0.35;
    min_dim / max_extent
}

fn project_point(
    x: f32,
    y: f32,
    z: f32,
    center: Pos2,
    scale: f32,
    camera: &OrbitCamera,
) -> Option<Pos2> {
    let (rx, ry, _rz) = rotate_orbit(x, y, z, camera.yaw, camera.pitch);
    let zoom_scale = scale / camera.distance.max(0.001);
    let px = center.x + (rx + camera.pan_x) * zoom_scale;
    let py = center.y - (ry + camera.pan_y) * zoom_scale;
    Some(Pos2::new(px, py))
}

fn rotate_orbit(x: f32, y: f32, z: f32, yaw: f32, pitch: f32) -> (f32, f32, f32) {
    let cos_y = yaw.cos();
    let sin_y = yaw.sin();
    let x1 = x * cos_y - z * sin_y;
    let z1 = x * sin_y + z * cos_y;

    let cos_p = pitch.cos();
    let sin_p = pitch.sin();
    let y2 = y * cos_p - z1 * sin_p;
    let z2 = y * sin_p + z1 * cos_p;

    (x1, y2, z2)
}

fn draw_grid(painter: &egui::Painter, rect: Rect, scale: f32, camera: &OrbitCamera) {
    let grid_color = Color32::from_rgb(30, 41, 59);
    let steps = 8;
    for i in -steps..=steps {
        let t = i as f32 / steps as f32 * 2.0;
        if let (Some(a), Some(b)) = (
            project_point(t, 0.0, -1.0, rect.center(), scale, camera),
            project_point(t, 0.0, 1.0, rect.center(), scale, camera),
        ) {
            painter.line_segment([a, b], egui::Stroke::new(1.0, grid_color));
        }
        if let (Some(a), Some(b)) = (
            project_point(-1.0, 0.0, t, rect.center(), scale, camera),
            project_point(1.0, 0.0, t, rect.center(), scale, camera),
        ) {
            painter.line_segment([a, b], egui::Stroke::new(1.0, grid_color));
        }
    }
}

fn draw_axes(painter: &egui::Painter, center: Pos2, scale: f32, camera: &OrbitCamera) {
    let origin = project_point(0.0, 0.0, 0.0, center, scale, camera).unwrap_or(center);
    let axes = [
        (1.0, 0.0, 0.0, Color32::from_rgb(239, 68, 68), "X"),
        (0.0, 1.0, 0.0, Color32::from_rgb(34, 197, 94), "Y"),
        (0.0, 0.0, 1.0, Color32::from_rgb(59, 130, 246), "Z"),
    ];
    for (x, y, z, color, label) in axes {
        if let Some(end) = project_point(x * 0.5, y * 0.5, z * 0.5, center, scale, camera) {
            painter.line_segment([origin, end], egui::Stroke::new(2.0, color));
            painter.text(end, egui::Align2::CENTER_CENTER, label, egui::FontId::proportional(11.0), color);
        }
    }
}
