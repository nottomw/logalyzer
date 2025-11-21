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

struct UserSettings {
    wrap_text: bool,
    autoscroll: bool,
    search_term: String,
    filter_term: String,
    file_path: Option<String>,
}

impl Default for UserSettings {
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

struct LogalyzerState {
    vertical_scroll_offset: f32,
}

impl Default for LogalyzerState {
    fn default() -> Self {
        Self {
            vertical_scroll_offset: 0.0,
        }
    }
}

struct LogalyzerGUI {
    user_settings: UserSettings,
    state: LogalyzerState,
}

impl Default for LogalyzerGUI {
    fn default() -> Self {
        Self {
            user_settings: UserSettings::default(),
            state: LogalyzerState::default(),
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
    // println!("Loading file: {}", path);

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
// TODO: stream support
// TODO: log format colors
// TODO: tokenizer colors

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
                            self.user_settings.file_path = Some(path.to_string_lossy().to_string());
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

                    let button_save_config = ui.button("Save config");
                    if button_save_config.clicked() {
                        println!("Save config button clicked");
                    }

                    let button_load_config = ui.button("Load config");
                    if button_load_config.clicked() {
                        println!("Load config button clicked");
                    }

                    ui.checkbox(&mut self.user_settings.wrap_text, "Wrap");
                    ui.checkbox(&mut self.user_settings.autoscroll, "Autoscroll");
                });

                ui.horizontal(|ui| {
                    egui::Grid::new("").show(ui, |ui| {
                        ui.label("Search:");
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::TextEdit::singleline(&mut self.user_settings.search_term),
                        );
                        ui.end_row();

                        ui.label("Filter:");
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::TextEdit::singleline(&mut self.user_settings.filter_term),
                        );
                        ui.end_row();
                    });
                });
            });

        let mut job = make_rich_text();
        let mut loaded_file_max_line_chars: usize = 0;
        let mut loaded_file_linecount: usize = 0;

        if self.user_settings.file_path.is_some() {
            // TODO: this loads the file every redraw
            let loaded_file_info = load_file(self.user_settings.file_path.clone().unwrap());
            if let Some(loaded_file) = loaded_file_info {
                job = loaded_file.layout_job;
                loaded_file_max_line_chars = loaded_file.content_max_line_chars;
                loaded_file_linecount = loaded_file.content_line_count;
            }
        }

        let mut line_numbers = String::new();
        for line_num in 1..=loaded_file_linecount {
            line_numbers.push_str(&format!("{}\n", line_num));
        }

        let _central_panel = egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_height(central_panel_height);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                let mut width_left_after_adding_line_numbers = 0.0;
                egui::ScrollArea::vertical()
                    .id_salt("line_numbers")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                    .vertical_scroll_offset(self.state.vertical_scroll_offset)
                    .show(ui, |ui| {
                        ui.label(line_numbers);
                        width_left_after_adding_line_numbers = ui.available_width();
                    });

                let scroll_area_width_max = if self.user_settings.wrap_text {
                    width_left_after_adding_line_numbers
                } else {
                    (loaded_file_max_line_chars as f32) * 8.0 + 50.0
                };

                let scroll_area = egui::ScrollArea::both()
                    .id_salt("log_file")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .max_width(scroll_area_width_max)
                    .show(ui, |ui| {
                        let mut text_wrapping = TextWrapping::default();
                        if self.user_settings.wrap_text {
                            text_wrapping.break_anywhere = true;
                        }

                        text_wrapping.max_width = scroll_area_width_max;
                        ui.set_width(scroll_area_width_max);

                        job.wrap = text_wrapping;

                        ui.add(egui::Label::new(job).wrap_mode(egui::TextWrapMode::Wrap));
                    });

                self.state.vertical_scroll_offset = scroll_area.state.offset.y;
            });
        });
    }
}
