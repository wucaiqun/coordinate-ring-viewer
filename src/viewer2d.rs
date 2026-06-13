use egui::{Align2, Color32, RichText, Ui};

const VIEW_CAPTION_SIZE: f32 = 14.0;
const TEXT_PRIMARY: Color32 = Color32::from_rgb(235, 235, 235);
use egui_plot::{Legend, Line, Plot, PlotPoint, PlotPoints, Points, Polygon, Text};
use crate::parser::{ParseResult2D, Ring2D};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickedPoint2D {
    pub ring_idx: usize,
    pub point_idx: usize,
}

const RING_COLORS: [Color32; 8] = [
    Color32::from_rgb(56, 189, 248),
    Color32::from_rgb(251, 146, 60),
    Color32::from_rgb(74, 222, 128),
    Color32::from_rgb(244, 114, 182),
    Color32::from_rgb(167, 139, 250),
    Color32::from_rgb(250, 204, 21),
    Color32::from_rgb(94, 234, 212),
    Color32::from_rgb(248, 113, 113),
];

pub fn ring_color(index: usize) -> Color32 {
    RING_COLORS[index % RING_COLORS.len()]
}

pub fn show_2d_view(
    ui: &mut Ui,
    data: &ParseResult2D,
    view_revision: u64,
    selected: Option<PickedPoint2D>,
    picked_out: &mut Option<PickedPoint2D>,
) -> egui::Response {
    ui.vertical(|ui| {
        ui.label(
            RichText::new(format!(
                "2D view - {} ring(s), {} point(s)",
                data.rings.len(),
                data.rings.iter().map(|r| r.points.len()).sum::<usize>()
            ))
            .size(VIEW_CAPTION_SIZE)
            .color(TEXT_PRIMARY),
        );
        ui.add_space(4.0);

        Plot::new(format!("geo_2d_plot_{view_revision}"))
            .allow_zoom(true)
            .allow_drag(true)
            .allow_scroll(true)
            .legend(Legend::default())
            .height(ui.available_height())
            .show(ui, |plot_ui| {
                for (idx, ring) in data.rings.iter().enumerate() {
                    draw_ring_2d(plot_ui, idx, ring, selected);
                }

                if plot_ui.response().clicked() {
                    if let Some(pointer) = plot_ui.pointer_coordinate() {
                        *picked_out = find_nearest_point_2d(data, pointer.x, pointer.y);
                    }
                }
            })
            .response
    })
    .inner
}

fn draw_ring_2d(
    plot_ui: &mut egui_plot::PlotUi,
    idx: usize,
    ring: &Ring2D,
    selected: Option<PickedPoint2D>,
) {
    if ring.points.is_empty() {
        return;
    }

    let color = ring_color(idx);
    let name = format!("Ring {}", idx + 1);

    plot_ui.points(
        Points::new(PlotPoints::from(
            ring.points.iter().map(|p| [p.lon, p.lat]).collect::<Vec<[f64; 2]>>(),
        ))
        .radius(4.0)
        .color(color)
        .name(format!("{name} points")),
    );

    if let Some(selected) = selected {
        if selected.ring_idx == idx {
            if let Some(point) = ring.points.get(selected.point_idx) {
                plot_ui.points(
                    Points::new(PlotPoints::from(vec![[point.lon, point.lat]]))
                        .radius(7.5)
                        .color(Color32::WHITE)
                        .name("Selected point"),
                );
                plot_ui.text(
                    Text::new(
                        PlotPoint::new(point.lon, point.lat),
                        format_coord_2d(point.lon, point.lat),
                    )
                    .color(Color32::WHITE)
                    .anchor(Align2::LEFT_BOTTOM)
                    .highlight(true),
                );
            }
        }
    }

    if ring.points.len() >= 2 {
        let mut line_coords: Vec<[f64; 2]> = ring.points.iter().map(|p| [p.lon, p.lat]).collect();
        if ring.points.len() >= 3 {
            if let Some(first) = ring.points.first() {
                line_coords.push([first.lon, first.lat]);
            }
        }
        plot_ui.line(
            Line::new(PlotPoints::from(line_coords))
                .color(color)
                .width(2.0)
                .name(name.clone()),
        );
    }

    if ring.points.len() >= 3 {
        let poly: PlotPoints = ring.points.iter().map(|p| [p.lon, p.lat]).collect();
        plot_ui.polygon(
            Polygon::new(poly)
                .fill_color(color.gamma_multiply(0.15))
                .stroke(egui::Stroke::NONE),
        );
    }
}

fn find_nearest_point_2d(data: &ParseResult2D, lon: f64, lat: f64) -> Option<PickedPoint2D> {
    let mut best: Option<(PickedPoint2D, f64)> = None;
    for (ring_idx, ring) in data.rings.iter().enumerate() {
        for (point_idx, point) in ring.points.iter().enumerate() {
            let dx = point.lon - lon;
            let dy = point.lat - lat;
            let dist2 = dx * dx + dy * dy;
            match best {
                Some((_, best_dist2)) if dist2 >= best_dist2 => {}
                _ => {
                    best = Some((PickedPoint2D { ring_idx, point_idx }, dist2));
                }
            }
        }
    }
    best.map(|(picked, _)| picked)
}

fn format_coord_2d(lon: f64, lat: f64) -> String {
    format!("{lon:.6} {lat:.6}")
}
