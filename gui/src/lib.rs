use core::f32;

use eframe::egui;
use egui::containers::scroll_area::ScrollBarVisibility;
use egui::text::{LayoutJob, TextWrapping};
use log_engine::user_settings::UserSettings;
use log_engine::*;

pub fn run_gui() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    let run_result = eframe::run_native(
        "Logalyzer",
        options,
        Box::new(|_cc| Ok(Box::new(LogalyzerGUI::default()) as Box<dyn eframe::App>)),
    );

    if run_result.is_err() {
        println!("Error running GUI: {:?}", run_result.err());
    }
}

struct LogalyzerState {
    vertical_scroll_offset: f32,
    opened_file: Option<OpenedFileMetadata>,
    log_job: LayoutJob,
    win_log_format_open: bool,
    panel_token_colors_open: bool,
    log_format_mode_selected: usize,
}

impl Default for LogalyzerState {
    fn default() -> Self {
        Self {
            vertical_scroll_offset: 0.0,
            opened_file: None,
            log_job: default_log_content(),
            win_log_format_open: false,
            panel_token_colors_open: false,
            log_format_mode_selected: 0, // 0 means manual regex
        }
    }
}

struct LogalyzerGUI {
    user_settings: UserSettings,
    user_settings_cached: UserSettings, // The only purpose of this is to detect changes and trigger repaints.
    user_settings_staging: UserSettings, // For editing, after OK/Apply is pressed this is copied to user_settings.
    // TODO: there is probably a bug here, related to canceling some different changes and then pressing OK in an unrelated window.
    state: LogalyzerState,
}

impl Default for LogalyzerGUI {
    fn default() -> Self {
        Self {
            user_settings: UserSettings::default(),
            user_settings_cached: UserSettings::default(),
            user_settings_staging: UserSettings::default(),
            state: LogalyzerState::default(),
        }
    }
}

impl eframe::App for LogalyzerGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let available_rect = ctx.available_rect();

        let bottom_panel_height = available_rect.height() * 0.2;
        let central_panel_height = available_rect.height() - bottom_panel_height;

        let _bottom_panel = egui::TopBottomPanel::bottom("controls")
            .max_height(bottom_panel_height)
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let button_file = ui.button("Open File");
                    if button_file.clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_file() {
                            println!("Selected file: {:?}", path);
                            self.user_settings.file_path = path.to_string_lossy().to_string();
                        }
                    }

                    let button_remote = ui.button("Open Stream");
                    if button_remote.clicked() {
                        println!("not implemented");
                    }

                    let button_log_format = ui.button("Log Format");
                    if button_log_format.clicked() {
                        self.state.win_log_format_open = true;
                    }

                    if self.state.win_log_format_open {
                        egui::Window::new("Log Format")
                            .auto_sized()
                            .show(ctx, |ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("Please select log format mode:");
                                        egui::ComboBox::from_id_salt("log_format_mode")
                                            .selected_text(
                                                match self.state.log_format_mode_selected {
                                                    0 => "Manual Regex",
                                                    1 => "[number.number] log message",
                                                    2 => "YYYY-MM-DD HH:MM:SS log message",
                                                    _ => "Manual Regex",
                                                },
                                            )
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut self.state.log_format_mode_selected,
                                                    0,
                                                    "Manual Regex",
                                                );
                                                ui.selectable_value(
                                                    &mut self.state.log_format_mode_selected,
                                                    1,
                                                    "[number.number] log message",
                                                );
                                                ui.selectable_value(
                                                    &mut self.state.log_format_mode_selected,
                                                    2,
                                                    "YYYY-MM-DD HH:MM:SS log message",
                                                );
                                            });
                                    });

                                    self.user_settings_staging.log_format.pattern = match {
                                        self.state.log_format_mode_selected
                                    } {
                                        0 => self.user_settings_staging.log_format.pattern.clone(),
                                        1 => r"^(\[\s*[0-9]*)(\.)([0-9]*\])(\s.*)$".to_string(),
                                        2 => r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})(\s+)(.*)$"
                                            .to_string(),
                                        _ => r"".to_string(), // impossible
                                    };

                                    let mut compiled_regex_valid = false;

                                    egui::Grid::new("log_format_grid").show(ui, |ui| {
                                        ui.label("Log Format Regex:");
                                        ui.add_sized(
                                            [400.0, 20.0],
                                            egui::TextEdit::singleline(
                                                &mut self.user_settings_staging.log_format.pattern,
                                            ),
                                        );
                                        ui.end_row();

                                        let compiled_regex = regex::Regex::new(
                                            &self.user_settings_staging.log_format.pattern,
                                        );
                                        compiled_regex_valid = compiled_regex.is_ok();
                                        if !compiled_regex_valid {
                                            ui.colored_label(
                                                egui::Color32::RED,
                                                "Invalid regex pattern",
                                            );
                                            ui.end_row();
                                        } else {
                                            ui.colored_label(egui::Color32::GREEN, "Regex valid");
                                            ui.end_row();

                                            let regex = compiled_regex.unwrap();
                                            let capture_group_count = regex.captures_len() - 1;

                                            for i in 0..capture_group_count {
                                                ui.label(format!("Group #{} Color:", i + 1));

                                                self.user_settings_staging
                                                    .log_format
                                                    .pattern_coloring
                                                    .resize(
                                                        capture_group_count,
                                                        egui::Color32::RED,
                                                    );

                                                ui.color_edit_button_srgba(
                                                    &mut self
                                                        .user_settings_staging
                                                        .log_format
                                                        .pattern_coloring[i],
                                                );
                                                ui.end_row();
                                            }
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        let button_ok = ui.add_enabled(
                                            compiled_regex_valid,
                                            egui::Button::new("OK"),
                                        );
                                        if button_ok.clicked() {
                                            self.state.win_log_format_open = false;
                                            self.user_settings.log_format =
                                                self.user_settings_staging.log_format.clone();
                                        }

                                        let button_apply = ui.add_enabled(
                                            compiled_regex_valid,
                                            egui::Button::new("Apply"),
                                        );
                                        if button_apply.clicked() {
                                            self.user_settings.log_format =
                                                self.user_settings_staging.log_format.clone();
                                        }

                                        let button_cancel = ui.button("Cancel");
                                        if button_cancel.clicked() {
                                            self.state.win_log_format_open = false;
                                        }
                                    });
                                });
                            });
                    }

                    let button_rules = ui.button("Token Rules");
                    if button_rules.clicked() {
                        self.state.panel_token_colors_open = !self.state.panel_token_colors_open;
                    }

                    let file_opened = self.state.opened_file.is_some();

                    let button_histogram =
                        ui.add_enabled(file_opened, egui::Button::new("Histogram"));
                    if button_histogram.clicked() {
                        println!("not implemented");
                        // TODO: implement histogram
                    }

                    let button_stats = ui.add_enabled(file_opened, egui::Button::new("Stats"));
                    if button_stats.clicked() {
                        println!("not implemented");
                        // TODO: implement stats
                    }

                    let button_save_config = ui.button("Save config");
                    if button_save_config.clicked() {
                        println!("not implemented");
                        // TODO: implement config saving
                    }

                    let button_load_config = ui.button("Load config");
                    if button_load_config.clicked() {
                        println!("not implemented");
                        // TODO: implement config loading
                    }

                    ui.add_enabled(
                        file_opened,
                        egui::Checkbox::new(&mut self.user_settings.wrap_text, "Wrap"),
                    );

                    ui.add_enabled(
                        file_opened,
                        egui::Checkbox::new(&mut self.user_settings.autoscroll, "Autoscroll"),
                    );
                });

                ui.horizontal(|ui| {
                    egui::Grid::new("").show(ui, |ui| {
                        ui.label("Search:");
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::TextEdit::singleline(&mut self.user_settings.search_term),
                        );
                        ui.checkbox(&mut self.user_settings.search_match_case, "Match Case");
                        ui.checkbox(&mut self.user_settings.search_whole_word, "Whole Word");
                        ui.end_row();

                        ui.label("Filter:");
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::TextEdit::singleline(&mut self.user_settings.filter_term),
                        );
                        ui.checkbox(&mut self.user_settings.filter_match_case, "Match Case");
                        ui.checkbox(&mut self.user_settings.filter_whole_word, "Whole Word");
                        ui.checkbox(&mut self.user_settings.filter_negative, "Negative");
                        ui.end_row();
                    });
                });
            });

        if self.state.panel_token_colors_open {
            egui::SidePanel::new(egui::panel::Side::Right, "tokens")
                .resizable(false)
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.heading("Token colors");

                    egui::Grid::new("tokens_grid").show(ui, |ui| {
                        for i in 0..self.user_settings_staging.token_colors.capacity() {
                            let token_color = &mut self.user_settings_staging.token_colors[i];

                            ui.label(format!("#{}:", i + 1));
                            ui.add_sized(
                                [100.0, 20.0],
                                egui::TextEdit::singleline(&mut token_color.0),
                            );
                            ui.color_edit_button_srgba(&mut token_color.1);
                            ui.end_row();
                        }
                    });

                    ui.horizontal(|ui| {
                        let button_apply = ui.button("Apply");
                        if button_apply.clicked() {
                            self.user_settings.token_colors =
                                self.user_settings_staging.token_colors.clone();
                        }

                        let button_close = ui.button("Close");
                        if button_close.clicked() {
                            self.state.panel_token_colors_open = false;
                        }
                    });
                });
        }

        // TODO: log job recalc should be offloaded to a separate thread
        if self.user_settings.file_path.is_empty() == false {
            if !self.state.opened_file.is_some()
                || self.state.opened_file.as_ref().unwrap().path != self.user_settings.file_path
            {
                // Reload file if it was requested, or the path has changed.
                let loaded_file_meta = load_file(&self.user_settings);
                self.state.opened_file = loaded_file_meta;

                if let Some(opened_file) = self.state.opened_file.as_ref() {
                    if let Some(file_job) = recalculate_log_job(opened_file, &self.user_settings) {
                        self.state.log_job = file_job;
                    }
                }
            } else {
                if self.user_settings != self.user_settings_cached {
                    self.user_settings_cached = self.user_settings.clone();
                    let opened_file = self.state.opened_file.as_ref().unwrap();
                    if let Some(file_job) = recalculate_log_job(opened_file, &self.user_settings) {
                        self.state.log_job = file_job;
                    }
                }
            }
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
                            ui.set_min_height(ui.available_height());

                            let layout_job_numbers = LayoutJob::simple_format(
                                line_numbers,
                                egui::TextFormat {
                                    font_id: self.user_settings.font.clone(),
                                    ..Default::default()
                                },
                            );
                            ui.label(layout_job_numbers);
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
                        ui.set_min_height(ui.available_height());

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
