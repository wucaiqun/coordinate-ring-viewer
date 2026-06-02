use std::sync::Arc;

use eframe::egui::{self, text::{CCursor, CCursorRange, LayoutJob, TextFormat}, Color32, FontId, RichText, ScrollArea, TextEdit, TextStyle, Visuals};
use crate::import::{format_import_status, merge_files, pick_coordinate_files, EXAMPLES_SAMPLE_PATHS};
use crate::parser::{parse_2d, parse_3d, ParseResult2D, ParseResult3D};
use crate::viewer2d::{show_2d_view, PickedPoint2D};
use crate::viewer3d::{show_3d_view, OrbitCamera, PickedPoint3D};

/// Primary labels and section titles.
const TEXT_PRIMARY: Color32 = Color32::from_rgb(235, 235, 235);
/// Secondary hints (smaller, softer).
const TEXT_MUTED: Color32 = Color32::from_gray(150);
const TEXT_FAINT: Color32 = Color32::from_gray(130);
const COLOR_OK: Color32 = Color32::from_rgb(74, 222, 128);
const COLOR_ERR: Color32 = Color32::from_rgb(251, 113, 133);
const COLOR_WARN: Color32 = Color32::from_rgb(251, 191, 36);
const COLOR_PICK_HIGHLIGHT: Color32 = Color32::from_rgb(30, 58, 138);

const FONT_BODY: f32 = 14.0;
const FONT_HINT: f32 = 12.0;
const FONT_SECTION: f32 = 15.0;
const FONT_STATUS: f32 = 13.0;

fn hint(text: impl Into<String>) -> RichText {
    RichText::new(text.into())
        .size(FONT_HINT)
        .color(TEXT_MUTED)
}

fn hint_faint(text: impl Into<String>) -> RichText {
    RichText::new(text.into())
        .size(FONT_HINT)
        .color(TEXT_FAINT)
}

fn section_title(text: impl Into<String>) -> RichText {
    RichText::new(text.into())
        .strong()
        .size(FONT_SECTION)
        .color(TEXT_PRIMARY)
}

fn body_label(text: impl Into<String>) -> RichText {
    RichText::new(text.into()).size(FONT_BODY).color(TEXT_PRIMARY)
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum CoordDimension {
    Dim2D,
    Dim3D,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ViewTab {
    Split,
    View2D,
    View3D,
}

pub struct GeoApp {
    input_2d: String,
    input_3d: String,
    parsed_2d: ParseResult2D,
    parsed_3d: ParseResult3D,
    camera: OrbitCamera,
    active_tab: ViewTab,
    show_errors: bool,
    input_revision: u64,
    view_revision: u64,
    status_message: Option<String>,
    status_is_error: bool,
    selected_2d: Option<PickedPoint2D>,
    selected_3d: Option<PickedPoint3D>,
    last_scrolled_2d: Option<PickedPoint2D>,
    last_scrolled_3d: Option<PickedPoint3D>,
}

impl GeoApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            input_2d: String::new(),
            input_3d: String::new(),
            parsed_2d: ParseResult2D::default(),
            parsed_3d: ParseResult3D::default(),
            camera: OrbitCamera::default(),
            active_tab: ViewTab::Split,
            show_errors: false,
            input_revision: 0,
            view_revision: 0,
            status_message: None,
            status_is_error: false,
            selected_2d: None,
            selected_3d: None,
            last_scrolled_2d: None,
            last_scrolled_3d: None,
        };
        app.camera.reset();
        app
    }

    fn refresh(&mut self) {
        self.parsed_2d = parse_2d(&self.input_2d);
        self.parsed_3d = parse_3d(&self.input_3d);
        self.show_errors =
            !self.parsed_2d.errors.is_empty() || !self.parsed_3d.errors.is_empty();
    }

    fn set_status(&mut self, message: String, is_error: bool) {
        self.status_message = Some(message);
        self.status_is_error = is_error;
    }

    fn apply_imported_text(&mut self, dim: CoordDimension, text: String, ctx: &egui::Context) {
        match dim {
            CoordDimension::Dim2D => self.input_2d = text,
            CoordDimension::Dim3D => self.input_3d = text,
        }
        self.input_revision = self.input_revision.wrapping_add(1);
        self.view_revision = self.view_revision.wrapping_add(1);
        self.camera.reset();
        self.selected_2d = None;
        self.selected_3d = None;
        self.last_scrolled_2d = None;
        self.last_scrolled_3d = None;
        self.refresh();
        ctx.request_repaint();
    }

    fn import_from_files(&mut self, dim: CoordDimension, ctx: &egui::Context) {
        let title = match dim {
            CoordDimension::Dim2D => "Import 2D coordinates from file(s)",
            CoordDimension::Dim3D => "Import 3D coordinates from file(s)",
        };

        let Some(paths) = pick_coordinate_files(title) else {
            return;
        };

        match merge_files(&paths) {
            Ok(text) => {
                let multi = paths.len() > 1;
                self.apply_imported_text(dim, text, ctx);
                self.set_status(format_import_status(&paths, multi), false);
            }
            Err(err) => self.set_status(err, true),
        }
    }

    fn clear_inputs(&mut self, ctx: &egui::Context) {
        self.input_2d.clear();
        self.input_3d.clear();
        self.input_revision = self.input_revision.wrapping_add(1);
        self.view_revision = self.view_revision.wrapping_add(1);
        self.camera.reset();
        self.selected_2d = None;
        self.selected_3d = None;
        self.last_scrolled_2d = None;
        self.last_scrolled_3d = None;
        self.refresh();
        self.set_status("Cleared 2D and 3D inputs.".to_string(), false);
        ctx.request_repaint();
    }
}

impl eframe::App for GeoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        setup_theme(ctx);

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Geo Ring Viewer").strong().size(20.0));
                ui.label(
                    RichText::new("Lat/lon ring visualizer")
                        .color(TEXT_MUTED)
                        .size(FONT_BODY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(egui::Button::new(RichText::new("Draw").strong()).min_size(egui::vec2(72.0, 28.0)))
                        .clicked()
                    {
                        self.refresh();
                        self.set_status("Plot updated.".to_string(), false);
                    }
                });
            });
            ui.add_space(6.0);
        });

        egui::SidePanel::left("input_panel")
            .resizable(true)
            .default_width(420.0)
            .min_width(320.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(section_title("Coordinates"));
                ui.label(hint_faint(format!("Sample files: {EXAMPLES_SAMPLE_PATHS}")));
                ui.add_space(6.0);

                if ui.button("Clear all inputs").clicked() {
                    self.clear_inputs(ctx);
                }

                if let Some(msg) = &self.status_message {
                    let color = if self.status_is_error {
                        COLOR_ERR
                    } else {
                        COLOR_OK
                    };
                    ui.label(RichText::new(msg).size(FONT_STATUS).color(color));
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(section_title("2D - lon lat"));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Import 2D from file(s)...").clicked() {
                        self.import_from_files(CoordDimension::Dim2D, ctx);
                    }
                });
                ui.label(hint(
                    "Select one or more .txt files. Multiple files are merged with ===============",
                ));
                ScrollArea::vertical()
                    .id_salt("input_2d")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        let selected_line_2d = selected_line_for_2d(&self.input_2d, self.selected_2d);
                        let input_id_2d = egui::Id::new("input_2d").with(self.input_revision);
                        let should_scroll_2d = self.selected_2d != self.last_scrolled_2d;
                        let mut layouter = make_line_highlight_layouter(selected_line_2d);
                        let mut output = TextEdit::multiline(&mut self.input_2d)
                            .id(input_id_2d)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .layouter(&mut layouter)
                            .show(ui);
                        if should_scroll_2d {
                            if let Some(line_idx) = selected_line_2d {
                                let char_index = char_index_for_line(&self.input_2d, line_idx);
                                let cursor = CCursor::new(char_index);
                                output.state.cursor.set_char_range(Some(CCursorRange::one(cursor)));
                                output.state.store(ctx, output.response.id);

                                // Put selected line at the top of the visible input area.
                                let line_rect = output
                                    .galley
                                    .pos_from_ccursor(cursor)
                                    .translate(output.galley_pos.to_vec2())
                                    .expand2(egui::vec2(0.0, 2.0));
                                ui.scroll_to_rect(line_rect, Some(egui::Align::Min));
                                ctx.request_repaint();
                            }
                            self.last_scrolled_2d = self.selected_2d;
                        }
                    });

                ui.add_space(12.0);
                ui.label(section_title("3D - lon lat altitude (m)"));
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Import 3D from file(s)...").clicked() {
                        self.import_from_files(CoordDimension::Dim3D, ctx);
                    }
                });
                ui.label(hint("Same merge rules as 2D import"));
                ScrollArea::vertical()
                    .id_salt("input_3d")
                    .max_height(200.0)
                    .show(ui, |ui| {
                        let selected_line_3d = selected_line_for_3d(&self.input_3d, self.selected_3d);
                        let input_id_3d = egui::Id::new("input_3d").with(self.input_revision);
                        let should_scroll_3d = self.selected_3d != self.last_scrolled_3d;
                        let mut layouter = make_line_highlight_layouter(selected_line_3d);
                        let mut output = TextEdit::multiline(&mut self.input_3d)
                            .id(input_id_3d)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(10)
                            .layouter(&mut layouter)
                            .show(ui);
                        if should_scroll_3d {
                            if let Some(line_idx) = selected_line_3d {
                                let char_index = char_index_for_line(&self.input_3d, line_idx);
                                let cursor = CCursor::new(char_index);
                                output.state.cursor.set_char_range(Some(CCursorRange::one(cursor)));
                                output.state.store(ctx, output.response.id);

                                // Put selected line at the top of the visible input area.
                                let line_rect = output
                                    .galley
                                    .pos_from_ccursor(cursor)
                                    .translate(output.galley_pos.to_vec2())
                                    .expand2(egui::vec2(0.0, 2.0));
                                ui.scroll_to_rect(line_rect, Some(egui::Align::Min));
                                ctx.request_repaint();
                            }
                            self.last_scrolled_3d = self.selected_3d;
                        }
                    });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(body_label(format!("2D: {} rings", self.parsed_2d.rings.len())));
                    ui.label(body_label(format!("3D: {} rings", self.parsed_3d.rings.len())));
                });

                if self.show_errors {
                    ui.add_space(8.0);
                    ui.collapsing("Parse warnings", |ui| {
                        for err in &self.parsed_2d.errors {
                            ui.label(
                                RichText::new(err)
                                    .size(FONT_BODY)
                                    .color(COLOR_WARN),
                            );
                        }
                        for err in &self.parsed_3d.errors {
                            ui.label(
                                RichText::new(err)
                                    .size(FONT_BODY)
                                    .color(COLOR_WARN),
                            );
                        }
                    });
                }

                ui.add_space(8.0);
                ui.label(section_title("Format"));
                ui.label(hint(
                    "- Separate values with space, comma, or semicolon\n\
                     - Lines starting with # or // are comments\n\
                     - Each ring needs at least 1 point; 3+ points auto-close",
                ));
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, ViewTab::Split, "Split");
                ui.selectable_value(&mut self.active_tab, ViewTab::View2D, "2D");
                ui.selectable_value(&mut self.active_tab, ViewTab::View3D, "3D");
            });
            ui.separator();

            match self.active_tab {
                ViewTab::Split => {
                    ui.columns(2, |cols| {
                        egui::Frame::group(cols[0].style()).show(&mut cols[0], |ui| {
                            let mut picked = None;
                            show_2d_view(
                                ui,
                                &self.parsed_2d,
                                self.view_revision,
                                self.selected_2d,
                                &mut picked,
                            );
                            if let Some(picked) = picked {
                                self.selected_2d = Some(picked);
                                ctx.request_repaint();
                            }
                        });
                        egui::Frame::group(cols[1].style()).show(&mut cols[1], |ui| {
                            let mut picked = None;
                            show_3d_view(
                                ui,
                                &self.parsed_3d,
                                &mut self.camera,
                                self.selected_3d,
                                &mut picked,
                            );
                            if let Some(picked) = picked {
                                self.selected_3d = Some(picked);
                                ctx.request_repaint();
                            }
                        });
                    });
                }
                ViewTab::View2D => {
                    let mut picked = None;
                    show_2d_view(
                        ui,
                        &self.parsed_2d,
                        self.view_revision,
                        self.selected_2d,
                        &mut picked,
                    );
                    if let Some(picked) = picked {
                        self.selected_2d = Some(picked);
                        ctx.request_repaint();
                    }
                }
                ViewTab::View3D => {
                    let mut picked = None;
                    show_3d_view(
                        ui,
                        &self.parsed_3d,
                        &mut self.camera,
                        self.selected_3d,
                        &mut picked,
                    );
                    if let Some(picked) = picked {
                        self.selected_3d = Some(picked);
                        ctx.request_repaint();
                    }
                }
            }
        });
    }
}

fn make_line_highlight_layouter(
    selected_line_index: Option<usize>,
) -> impl FnMut(&egui::Ui, &str, f32) -> Arc<egui::Galley> {
    move |ui, text, wrap_width| {
        let mut job = LayoutJob::default();
        job.wrap.max_width = wrap_width;
        let default_format = TextFormat {
            font_id: FontId::monospace(13.0),
            color: TEXT_PRIMARY,
            ..Default::default()
        };
        let highlighted_format = TextFormat {
            font_id: FontId::monospace(13.0),
            color: TEXT_PRIMARY,
            background: COLOR_PICK_HIGHLIGHT,
            ..Default::default()
        };

        if text.is_empty() {
            job.append("", 0.0, default_format);
        } else {
            let lines: Vec<&str> = text.split('\n').collect();
            for (line_idx, line) in lines.iter().enumerate() {
                let fmt = if Some(line_idx) == selected_line_index {
                    highlighted_format.clone()
                } else {
                    default_format.clone()
                };
                if line_idx + 1 < lines.len() {
                    job.append(&((*line).to_owned() + "\n"), 0.0, fmt);
                } else {
                    job.append(line, 0.0, fmt);
                }
            }
        }

        ui.ctx().fonts(|fonts| fonts.layout_job(job))
    }
}

fn selected_line_for_2d(input: &str, selected: Option<PickedPoint2D>) -> Option<usize> {
    selected.and_then(|picked| selected_line_for_dims(input, picked.ring_idx, picked.point_idx, 2))
}

fn selected_line_for_3d(input: &str, selected: Option<PickedPoint3D>) -> Option<usize> {
    selected.and_then(|picked| selected_line_for_dims(input, picked.ring_idx, picked.point_idx, 3))
}

fn selected_line_for_dims(
    input: &str,
    target_ring_idx: usize,
    target_point_idx: usize,
    dims: usize,
) -> Option<usize> {
    let mut ring_idx = 0usize;
    let mut point_idx = 0usize;
    let mut has_point_in_ring = false;

    for (line_idx, raw_line) in input.lines().enumerate() {
        let stripped = strip_inline_comment(raw_line).trim();
        if stripped.is_empty() {
            continue;
        }
        if stripped == "===============" {
            if has_point_in_ring {
                ring_idx += 1;
                point_idx = 0;
                has_point_in_ring = false;
            }
            continue;
        }

        if looks_like_point_line(stripped, dims) {
            if ring_idx == target_ring_idx && point_idx == target_point_idx {
                return Some(line_idx);
            }
            point_idx += 1;
            has_point_in_ring = true;
        }
    }

    None
}

fn strip_inline_comment(line: &str) -> &str {
    line.split_once('#')
        .or_else(|| line.split_once("//"))
        .map(|(before, _)| before)
        .unwrap_or(line)
}

fn looks_like_point_line(line: &str, dims: usize) -> bool {
    let count = line
        .split(|c: char| c == ',' || c.is_whitespace() || c == ';' || c == '\t')
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse::<f64>().ok())
        .count();
    count >= dims
}

fn char_index_for_line(text: &str, target_line_idx: usize) -> usize {
    let mut char_index = 0usize;
    for (line_idx, line) in text.split('\n').enumerate() {
        if line_idx == target_line_idx {
            return char_index;
        }
        char_index += line.chars().count() + 1;
    }
    char_index
}

fn setup_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(17, 24, 39);
    visuals.window_fill = Color32::from_rgb(15, 23, 42);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 41, 59);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 41, 59);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(51, 65, 85);
    visuals.widgets.active.bg_fill = Color32::from_rgb(14, 165, 233);
    visuals.selection.bg_fill = Color32::from_rgb(14, 165, 233).gamma_multiply(0.35);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.text_styles.insert(TextStyle::Body, FontId::proportional(FONT_BODY));
    style.text_styles.insert(TextStyle::Button, FontId::proportional(FONT_BODY));
    style.text_styles.insert(TextStyle::Monospace, FontId::monospace(13.0));
    ctx.set_style(style);
}
