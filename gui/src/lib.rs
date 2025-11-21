use eframe::egui;
use egui::containers::scroll_area::ScrollBarVisibility;
use egui::text::{LayoutJob, TextWrapping};
use log_engine::*;

pub fn run_gui() {
    let options = eframe::NativeOptions::default();
    let _run_result = eframe::run_native(
        "Logalyzer",
        options,
        Box::new(|_cc| Ok(Box::new(LogalyzerGUI::default()) as Box<dyn eframe::App>)),
    );

    // TODO: return error code
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
    opened_file: Option<OpenedFileMetadata>,
    log_job: LayoutJob,
}

impl Default for LogalyzerState {
    fn default() -> Self {
        Self {
            vertical_scroll_offset: 0.0,
            opened_file: None,
            log_job: make_rich_text(),
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

        if self.user_settings.file_path.is_some() && !self.state.opened_file.is_some() {
            let (file_job, loaded_file_meta) =
                load_file(self.user_settings.file_path.clone().unwrap());

            self.state.log_job = file_job.clone();
            self.state.opened_file = loaded_file_meta;
        }

        let _central_panel = egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_height(central_panel_height);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                let mut width_left_after_adding_line_numbers = ui.available_width();

                let mut scroll_area_width_max = width_left_after_adding_line_numbers;

                if self.state.opened_file.is_some() {
                    let opened_file = self.state.opened_file.as_ref().unwrap();

                    let mut line_numbers = String::new();
                    for line_num in 1..=opened_file.content_line_count {
                        line_numbers.push_str(&format!("{}\n", line_num));
                    }

                    egui::ScrollArea::vertical()
                        .id_salt("line_numbers")
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                        .vertical_scroll_offset(self.state.vertical_scroll_offset)
                        .show(ui, |ui| {
                            ui.label(line_numbers);
                            width_left_after_adding_line_numbers = ui.available_width();
                        });

                    scroll_area_width_max = if self.user_settings.wrap_text {
                        width_left_after_adding_line_numbers
                    } else {
                        (opened_file.content_max_line_chars as f32) * 8.0 + 50.0
                    };
                }

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

                        self.state.log_job.wrap = text_wrapping;

                        ui.add(
                            egui::Label::new(self.state.log_job.clone())
                                .wrap_mode(egui::TextWrapMode::Wrap),
                        );
                    });

                self.state.vertical_scroll_offset = scroll_area.state.offset.y;
            });
        });
    }
}
