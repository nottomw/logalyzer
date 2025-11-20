use eframe::egui;
use egui::containers::scroll_area::ScrollBarVisibility;
use egui::{text::{LayoutJob, TextFormat, TextWrapping}, Color32, FontId};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Logalyzer",
        options,
        Box::new(|_cc| Ok(Box::new(LogalyzerGUI::default()) as Box<dyn eframe::App>)),
    )
}

fn make_rich_text() -> LayoutJob {
    let mut job = LayoutJob::default();

    job.append(
        "This is some ",
        0.0,
        TextFormat::default(),
    );

    job.append(
        "highlighted ",
        0.0,
        TextFormat {
            background: Color32::YELLOW,
            font_id: FontId::default(),
            color: Color32::BLACK,
            ..Default::default()
        },
    );

    job.append(
        "text with different background colors.",
        0.0,
        TextFormat::default(),
    );

    job
}

struct LogalyzerGUI {
    wrap_text: bool,
    autoscroll: bool,
    search_term: String,
    grep_term: String,
}

impl Default for LogalyzerGUI {
    fn default() -> Self {
        Self {
            wrap_text: true,
            autoscroll: false,
            search_term: String::new(),
            grep_term: String::new(),
        }
    }
}

// TODO: lua
// TODO: loading files / reading remotely with a specified command

impl eframe::App for LogalyzerGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        let central_panel = egui::CentralPanel::default();

        central_panel.show(ctx, |ui| {
            let available_rect = ui.ctx().available_rect();
            let total_height = available_rect.height();

            let sep_height = 6.0;
            let height_80 = total_height * 0.8 - sep_height / 2.0;
            let _height_20 = total_height * 0.2 - sep_height / 2.0;

            let upper_rect = egui::Rect::from_min_size(
                available_rect.min,
                egui::vec2(available_rect.width(), height_80),
            );

            let upper_area_builder = egui::UiBuilder::new().max_rect(upper_rect);
            ui.scope_builder(upper_area_builder, |ui| {
                egui::ScrollArea::both()
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .max_height(height_80)
                    .show(ui, |ui| {
                        ui.set_min_height(height_80);

                        let mut job = make_rich_text();
                        let mut text_wrapping = TextWrapping::default();
                        if self.wrap_text {
                            text_wrapping.max_width = ui.available_width();
                            ui.set_min_width(ui.available_width());
                        } else {
                            text_wrapping.max_width = f32::INFINITY;
                            ui.set_min_width(f32::INFINITY);
                        }
            
                        job.wrap = text_wrapping;
                        ui.label(job);
                    });
            });

            ui.separator();

            let lower_rect = egui::Rect::from_min_max(
                egui::pos2(available_rect.min.x, available_rect.min.y + height_80 + sep_height),
                available_rect.max,
            );

            let lower_area_builder = egui::UiBuilder::new().max_rect(lower_rect);
            ui.scope_builder(lower_area_builder, |ui| {
                ui.horizontal(|ui| {
                    let _ = ui.button("file");
                    let _ = ui.button("remote");
                    let _ = ui.button("rules");

                    ui.checkbox(&mut self.wrap_text, "Enable line wrapping");
                    ui.checkbox(&mut self.autoscroll, "Autoscroll");    
                });

                ui.horizontal(|ui| {
                    ui.label("search:");
                    ui.text_edit_singleline(&mut self.search_term);
                });

                ui.horizontal(|ui| {
                    ui.label("grep:");
                    ui.text_edit_singleline(&mut self.grep_term);
                });
            });
        });
    }
}
