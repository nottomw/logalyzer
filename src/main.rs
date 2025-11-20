use eframe::egui;
use egui::containers::scroll_area::ScrollBarVisibility;
use egui::{
    Color32, FontId,
    text::{LayoutJob, TextFormat, TextWrapping},
};

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

    job.append("This is some ", 0.0, TextFormat::default());

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
    filter_term: String,
}

impl Default for LogalyzerGUI {
    fn default() -> Self {
        Self {
            wrap_text: true,
            autoscroll: false,
            search_term: String::new(),
            filter_term: String::new(),
        }
    }
}

// TODO: lua
// TODO: loading files / reading remotely with a specified command

impl eframe::App for LogalyzerGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let available_rect = ctx.available_rect();

        let bottom_panel_height = available_rect.height() * 0.2;
        let central_panel_height = available_rect.height() - bottom_panel_height;

        let _bottom_panel = egui::TopBottomPanel::bottom("controls")
            .exact_height(bottom_panel_height)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let button_file = ui.button("File");
                    if button_file.clicked() {
                        println!("File button clicked");
                    }

                    let button_remote = ui.button("Remote");
                    if button_remote.clicked() {
                        println!("Remote button clicked");
                    }

                    let button_rules = ui.button("Rules");
                    if button_rules.clicked() {
                        println!("Rules button clicked");
                    }

                    ui.checkbox(&mut self.wrap_text, "Enable line wrapping");
                    ui.checkbox(&mut self.autoscroll, "Autoscroll");
                });

                ui.horizontal(|ui| {
                    egui::Grid::new("").show(ui, |ui| {
                        ui.label("Search:");
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::TextEdit::singleline(&mut self.search_term),
                        );
                        ui.end_row();

                        ui.label("Filter:");
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::TextEdit::singleline(&mut self.filter_term),
                        );
                        ui.end_row();
                    });
                });
            });

        let _central_panel = egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    ui.set_min_height(central_panel_height);

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
    }
}
