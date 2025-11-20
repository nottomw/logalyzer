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

struct LogalyzerGUI {
    wrap_text: bool,
    autoscroll: bool,
    search_term: String,
    filter_term: String,
    file_path: Option<String>,
}

impl Default for LogalyzerGUI {
    fn default() -> Self {
        Self {
            wrap_text: true,
            autoscroll: false,
            search_term: String::new(),
            filter_term: String::new(),
            file_path: None,
        }
    }
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

struct LoadedFile {
    layout_job: LayoutJob,
    content_max_line_chars: usize,
    content_line_count: usize,
}

fn load_file(path: String) -> Option<LoadedFile> {
    let read_result = std::fs::read_to_string(&path);
    if read_result.is_err() {
        return None;
    }

    let file_content = read_result.unwrap();

    let text_format = TextFormat {
        font_id: FontId::monospace(12.0),
        ..Default::default()
    };

    let mut job = LayoutJob::default();
    job.append(&file_content, 0.0, text_format);

    Some(LoadedFile {
        layout_job: job,
        content_max_line_chars: file_content
            .lines()
            .map(|line| line.len())
            .max()
            .unwrap_or(0),
        content_line_count: file_content.lines().count(),
    })
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
                    let button_file = ui.button("Open File");
                    if button_file.clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            println!("Selected file: {:?}", path);
                            self.file_path = Some(path.to_string_lossy().to_string());
                        }
                    }

                    let button_remote = ui.button("Open Stream");
                    if button_remote.clicked() {
                        println!("Open stream button clicked");
                    }

                    let button_log_format = ui.button("Log Format");
                    if button_log_format.clicked() {
                        println!("Log Format button clicked");
                    }

                    let button_rules = ui.button("Token Rules");
                    if button_rules.clicked() {
                        println!("Token rules button clicked");
                    }

                    ui.checkbox(&mut self.wrap_text, "Wrap");
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
                    let mut desired_width: f32 = ui.available_width();

                    if self.file_path.is_some() {
                        let loaded_file_info = load_file(self.file_path.clone().unwrap());
                        if let Some(loaded_file) = loaded_file_info {
                            job = loaded_file.layout_job;
                            desired_width = loaded_file.content_max_line_chars as f32 * 8.0;
                            desired_width += 20.0; // some padding for char rendering
                        }
                    }

                    let mut text_wrapping = TextWrapping::default();
                    if self.wrap_text {
                        text_wrapping.max_width = ui.available_width();
                        ui.set_min_width(ui.available_width());
                    } else {
                        text_wrapping.max_width = desired_width;
                        ui.set_min_width(desired_width);
                    }

                    job.wrap = text_wrapping;
                    ui.label(job);
                });
        });
    }
}
