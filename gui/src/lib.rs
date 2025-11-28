use core::f32;

use eframe::egui;
use egui::containers::scroll_area::ScrollBarVisibility;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Vec2, scroll_area};
use log_engine::OpenedFileMetadata;
use log_engine::user_settings::UserSettings;
use std::path::Path;

pub fn run_gui(args: Vec<String>) {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    let run_result = eframe::run_native(
        "Logalyzer",
        options,
        Box::new(|_cc| Ok(Box::new(LogalyzerGUI::new(args)) as Box<dyn eframe::App>)),
    );

    if run_result.is_err() {
        println!("Error running GUI: {:?}", run_result.err());
    }
}

enum FocusRequests {
    None,
    Search,
    Filter,
}

#[derive(Default)]
struct AddCommentRequest {
    line_no: usize,
    comment_text: String,
}

struct LogalyzerState {
    vertical_scroll_offset: f32,
    opened_file: Option<OpenedFileMetadata>,
    line_no_jobs: Vec<LayoutJob>,
    log_jobs: Vec<LayoutJob>,
    search_found: Vec<log_engine::PointOfInterest>,
    search_found_showing_index: usize,
    search_found_last_shown_index: Option<usize>,
    win_log_format_open: bool,
    panel_token_colors_open: bool,
    log_format_mode_selected: usize,
    lines_wrapped: usize,
    log_scroll_area_width: f32,
    focus_request: FocusRequests,
    add_comment_request: Option<AddCommentRequest>,
    add_comment_window_open: bool,
    visible_line_offsets: log_engine::VisibleLineOffsets,
}

impl Default for LogalyzerState {
    fn default() -> Self {
        Self {
            vertical_scroll_offset: 0.0,
            opened_file: None,
            line_no_jobs: vec![LayoutJob::default()],
            log_jobs: vec![log_engine::default_log_content()],
            search_found: Vec::new(),
            search_found_showing_index: 0,
            search_found_last_shown_index: None,
            win_log_format_open: false,
            panel_token_colors_open: false,
            log_format_mode_selected: 0, // 0 means manual regex
            lines_wrapped: 0,
            log_scroll_area_width: 0.0,
            focus_request: FocusRequests::None,
            add_comment_request: None,
            add_comment_window_open: false,
            visible_line_offsets: log_engine::VisibleLineOffsets::default(),
        }
    }
}

struct LogalyzerGUI {
    user_settings: UserSettings,
    user_settings_cached: UserSettings, // The only purpose of this is to detect changes and trigger repaints.
    user_settings_staging: UserSettings, // For editing, after OK/Apply is pressed part of this is copied to user_settings.
    state: LogalyzerState,
    scroll_sources_allowed: scroll_area::ScrollSource,
}

impl LogalyzerGUI {
    fn new(args: Vec<String>) -> Self {
        let mut new_self = Self::default();

        // For now processing cmdline args manually here as the params are super simple.
        for arg in args.iter().skip(1) {
            match arg.as_str() {
                "--help" | "-h" => {
                    println!("Logalyzer help:");
                    println!("--file=<path> Specify the log file to open.");
                    println!("--config=<path> Specify the configuration file to load.");
                    std::process::exit(0);
                }
                _ => {
                    if arg.starts_with("--file=") {
                        let file_path = arg.trim_start_matches("--file=");
                        new_self.user_settings.file_path = file_path.to_string();
                    }

                    if arg.starts_with("--config=") {
                        let config_path_str = arg.trim_start_matches("--config=");
                        let config_path = Path::new(config_path_str);
                        let user_settings_res = log_engine::configuration_load(config_path);
                        if let Ok(loaded_user_settings) = user_settings_res {
                            let orig_file_path = new_self.user_settings.file_path.clone();

                            {
                                new_self.user_settings = loaded_user_settings.clone();
                                new_self.user_settings_staging = loaded_user_settings;
                            }

                            // Preserve currently opened file path.
                            new_self.user_settings.file_path = orig_file_path.clone();
                            new_self.user_settings_staging.file_path = orig_file_path;
                        }
                    }
                }
            }
        }

        new_self
    }

    fn check_keyboard_shortcuts(&mut self, ui: &egui::Ui) {
        // Ctrl + F => focus search box
        // Ctrl + G => focus filter box
        // Ctrl + T => open tokens panel

        let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
        if ctrl_pressed {
            if ui.input(|i| i.key_pressed(egui::Key::F)) {
                self.state.focus_request = FocusRequests::Search;
            }

            if ui.input(|i| i.key_pressed(egui::Key::I)) {
                self.state.focus_request = FocusRequests::Filter;
            }

            if ui.input(|i| i.key_pressed(egui::Key::T)) {
                self.state.panel_token_colors_open = !self.state.panel_token_colors_open;
            }
        }
    }

    fn get_scroll_delta_based_on_keypress(
        &self,
        ctx: &egui::Context,
        ui: &egui::Ui,
        height: f32,
        width: f32,
    ) -> egui::Vec2 {
        let mut scroll_delta = egui::Vec2::ZERO;
        let mut anything_focused = false;

        ctx.memory(|mem| {
            anything_focused = mem.focused().is_some();
        });

        if anything_focused {
            return scroll_delta;
        }

        // These should be pretty big steps, so the user can navigate quickly.
        let scroll_delta_vertical = height * 0.4;
        let scroll_delta_horizontal = width * 0.3;

        if !anything_focused {
            if ui.input(|i| i.key_pressed(egui::Key::A)) {
                scroll_delta += egui::vec2(scroll_delta_horizontal, 0.0);
            }

            if ui.input(|i| i.key_pressed(egui::Key::D)) {
                scroll_delta += egui::vec2(-scroll_delta_horizontal, 0.0);
            }

            if ui.input(|i| i.key_pressed(egui::Key::W)) {
                scroll_delta += egui::vec2(0.0, scroll_delta_vertical);
            }

            if ui.input(|i| i.key_pressed(egui::Key::S)) {
                scroll_delta += egui::vec2(0.0, -scroll_delta_vertical);
            }
        }

        scroll_delta
    }

    fn determine_wrapping(&self, ctx: &egui::Context, ui: &egui::Ui, row_index: usize) -> usize {
        let mut line_wrapped_by = 0;

        // This is a pretty costly operation, could be cached.

        if self.user_settings.wrap_text {
            if let Some(job) = self.state.log_jobs.get(row_index) {
                let mut job_with_wrapping = job.clone();
                job_with_wrapping.wrap = TextWrapping {
                    break_anywhere: false,
                    max_width: if self.state.log_scroll_area_width == 0.0 {
                        ui.available_width() - 1.0
                    } else {
                        self.state.log_scroll_area_width
                    },
                    ..Default::default()
                };

                let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job_with_wrapping.clone()));
                let wrap_amount = galley.rows.len();
                line_wrapped_by = wrap_amount - 1;
            }
        }

        line_wrapped_by
    }

    fn show_log_format_window(&mut self, ctx: &egui::Context) {
        egui::Window::new("Log Format")
                .auto_sized()
                .collapsible(false)
                .open(&mut self.state.win_log_format_open) // this controls whether the window is open, but also shows the "X" to close the window...
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Please select log format mode:");
                            egui::ComboBox::from_id_salt("log_format_mode")
                                .selected_text(match self.state.log_format_mode_selected {
                                    0 => "Manual Regex",
                                    1 => "[number.number] log message",
                                    2 => "YYYY-MM-DD HH:MM:SS log message",
                                    _ => "Manual Regex",
                                })
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

                        ui.add_space(10.0);
                        ui.label("If you are defining your own regex, please make sure it has captures for each character in\nthe log line, as anything not captured will be removed.");
                        ui.add_space(5.0);
                        ui.label("Use transparency setting in color picker for groups you don't want to highlight.");
                        ui.add_space(10.0);

                        self.user_settings_staging.log_format.pattern = match {
                            self.state.log_format_mode_selected
                        } {
                            0 => self.user_settings_staging.log_format.pattern.clone(),
                            1 => r"^(\[\s*[0-9]*)(\.)([0-9]*\])(\s.*)$".to_string(),
                            2 => r"^(\d{4}-\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})(\s+)(.*)$".to_string(),
                            _ => r"".to_string(), // impossible
                        };

                        ui.horizontal(|ui| {
                            ui.label("Log Format Regex:");
                            ui.add_sized(
                                [400.0, 20.0],
                                egui::TextEdit::singleline(
                                    &mut self.user_settings_staging.log_format.pattern,
                                ),
                            );
                        });

                        let compiled_regex =
                            regex::Regex::new(&self.user_settings_staging.log_format.pattern);
                        let compiled_regex_valid = compiled_regex.is_ok();

                        if !self.user_settings_staging.log_format.pattern.is_empty() {
                            if !compiled_regex_valid {
                                ui.colored_label(egui::Color32::RED, "Regex invalid!");
                            } else {
                                ui.colored_label(egui::Color32::GREEN, "Regex valid.");
                            }
                        }

                        egui::Grid::new("log_format_grid").show(ui, |ui| {
                            if !self.user_settings_staging.log_format.pattern.is_empty() {
                                if compiled_regex_valid {
                                    let regex = compiled_regex.unwrap();
                                    let capture_group_count = regex.captures_len() - 1;

                                    self.user_settings_staging
                                            .log_format
                                            .pattern_coloring
                                            .resize(capture_group_count, egui::Color32::RED);

                                    self.user_settings_staging
                                        .log_format
                                        .pattern_coloring_text
                                        .resize(capture_group_count, egui::Color32::GRAY);

                                    self.user_settings_staging
                                        .log_format
                                        .pattern_coloring_text_use_original
                                        .resize(capture_group_count, true);

                                    for i in 0..capture_group_count {
                                        ui.label(format!("Group #{}:", i + 1));
                                        ui.label(format!("Background Color:"));

                                        ui.color_edit_button_srgba(
                                            &mut self
                                                .user_settings_staging
                                                .log_format
                                                .pattern_coloring[i],
                                        );

                                        ui.label("Text Color:");

                                        ui.color_edit_button_srgba(
                                            &mut self
                                                .user_settings_staging
                                                .log_format
                                                .pattern_coloring_text[i],
                                        );

                                        ui.checkbox(
                                            &mut self.user_settings_staging
                                                .log_format
                                                .pattern_coloring_text_use_original[i],
                                            "Use original text color",
                                        );

                                        ui.end_row();
                                    }
                                }
                            }
                        });

                        ui.horizontal(|ui| {
                            let button_ok =
                                ui.add_enabled(compiled_regex_valid, egui::Button::new("OK"));
                            if button_ok.clicked() {
                                self.user_settings.log_format =
                                    self.user_settings_staging.log_format.clone();
                                    ui.close_kind(egui::UiKind::Window)
                            }

                            let button_apply =
                                ui.add_enabled(compiled_regex_valid, egui::Button::new("Apply"));
                            if button_apply.clicked() {
                                self.user_settings.log_format =
                                    self.user_settings_staging.log_format.clone();
                            }

                            let button_cancel = ui.button("Cancel");
                            if button_cancel.clicked() {
                                ui.close_kind(egui::UiKind::Window)
                            }
                        });
                    });
                });
    }

    fn show_bottom_panel_first_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let button_file = ui.button("Open File");
            if button_file.clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    println!("Selected file: {:?}", path);
                    self.user_settings.file_path = path.to_string_lossy().to_string();
                }
            }

            let button_stream = ui.button("Open Stream");
            if button_stream.clicked() {
                // TODO: implement stream support
                println!("not implemented");
            }

            let button_log_format = ui.button("Log Format");
            if button_log_format.clicked() {
                self.state.win_log_format_open = true;
            }

            let button_rules = ui.button("Token Rules");
            if button_rules.clicked() {
                self.state.panel_token_colors_open = !self.state.panel_token_colors_open;
            }

            let file_opened = self.state.opened_file.is_some();

            let button_histogram = ui.add_enabled(file_opened, egui::Button::new("Histogram"));
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
                let selected_save_file = rfd::FileDialog::new()
                    .add_filter("Logalyzer Config", &["logalyzercfg"])
                    .save_file();
                if let Some(path) = selected_save_file {
                    log_engine::configuration_save(&path, &self.user_settings);
                }
            }

            let button_load_config = ui.button("Load config");
            if button_load_config.clicked() {
                let selected_load_file = rfd::FileDialog::new()
                    .add_filter("Logalyzer Config", &["logalyzercfg"])
                    .pick_file();

                if let Some(path) = selected_load_file {
                    let user_settings_res = log_engine::configuration_load(&path);
                    if let Ok(loaded_user_settings) = user_settings_res {
                        let orig_file_path = self.user_settings.file_path.clone();

                        {
                            self.user_settings = loaded_user_settings.clone();
                            self.user_settings_staging = loaded_user_settings;
                        }

                        // Preserve currently opened file path.
                        self.user_settings.file_path = orig_file_path.clone();
                        self.user_settings_staging.file_path = orig_file_path;
                    }
                }
            }

            ui.add_enabled(
                file_opened,
                egui::Checkbox::new(&mut self.user_settings.wrap_text, "Wrap"),
            );

            ui.add_enabled(
                false, // This should be on only if a stream is opened.
                egui::Checkbox::new(&mut self.user_settings.autoscroll, "Autoscroll"),
            );

            ui.add_enabled(
                file_opened,
                egui::Checkbox::new(&mut self.user_settings.comments_visible, "Comments"),
            );
        });
    }

    fn show_bottom_panel_search_and_filter(&mut self, ui: &mut egui::Ui) {
        let search_and_filter_label_size = Vec2::new(80.0, 20.0);
        let search_and_filter_input_size = Vec2::new(300.0, 20.0);

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.add_sized(search_and_filter_label_size, egui::Label::new("Search:"));
                let textedit_search = ui.add_sized(
                    search_and_filter_input_size,
                    egui::TextEdit::singleline(&mut self.user_settings.search_term)
                        .id_salt("search_input"),
                );

                if let FocusRequests::Search = self.state.focus_request {
                    textedit_search.request_focus();
                    self.state.focus_request = FocusRequests::None;
                }

                ui.checkbox(&mut self.user_settings.search_match_case, "Match Case");
                ui.checkbox(&mut self.user_settings.search_whole_word, "Whole Word");

                let search_prev_button = ui.add_enabled(
                    !self.state.search_found.is_empty(),
                    egui::Button::new("Previous"),
                );
                if search_prev_button.clicked() {
                    self.state.search_found_showing_index =
                        if self.state.search_found_showing_index == 0 {
                            self.state.search_found.len() - 1
                        } else {
                            self.state.search_found_showing_index - 1
                        }
                }

                let search_next_button = ui.add_enabled(
                    !self.state.search_found.is_empty(),
                    egui::Button::new("Next"),
                );
                if search_next_button.clicked() {
                    self.state.search_found_showing_index =
                        (self.state.search_found_showing_index + 1) % self.state.search_found.len();
                }

                if !self.state.search_found.is_empty() {
                    ui.label(format!(
                        "Result {} of {}",
                        self.state.search_found_showing_index + 1,
                        self.state.search_found.len()
                    ));
                }
            });

            ui.horizontal(|ui| {
                ui.add_sized(search_and_filter_label_size, egui::Label::new("Filter:"));
                let textedit_filter = ui.add_sized(
                    search_and_filter_input_size,
                    egui::TextEdit::singleline(&mut self.user_settings.filter_term)
                        .id_salt("filter_input"),
                );

                if let FocusRequests::Filter = self.state.focus_request {
                    textedit_filter.request_focus();
                    self.state.focus_request = FocusRequests::None;
                }

                ui.checkbox(&mut self.user_settings.filter_match_case, "Match Case");
                ui.checkbox(&mut self.user_settings.filter_whole_word, "Whole Word");
                ui.checkbox(&mut self.user_settings.filter_negative, "Negative");
                // TODO: extended filtering: && and || support maybe
                // TODO: maybe option to show N lines before/after match
            });
        });
    }

    fn show_token_colors_panel(&mut self, ctx: &egui::Context) {
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
    }

    fn recalculate_logfile_display(&mut self) {
        // TODO: log job recalc should be offloaded to a separate thread
        if self.user_settings.file_path.is_empty() == false {
            if !self.state.opened_file.is_some()
                || self.state.opened_file.as_ref().unwrap().path != self.user_settings.file_path
            {
                // Reload file if it was requested, or the path has changed.
                let loaded_file_meta = log_engine::load_file(&self.user_settings);
                self.state.opened_file = loaded_file_meta;

                if let Some(opened_file) = self.state.opened_file.as_mut() {
                    if let Some((line_no_jobs, file_jobs, _, _)) =
                        log_engine::recalculate_log_job(opened_file, &self.user_settings)
                    {
                        self.state.line_no_jobs = line_no_jobs;
                        self.state.log_jobs = file_jobs;
                        self.state.search_found = Vec::new();
                        self.state.search_found_showing_index = 0;
                        self.state.search_found_last_shown_index = None;
                    }
                }
            } else {
                if self.user_settings != self.user_settings_cached {
                    self.user_settings_cached = self.user_settings.clone();
                    let opened_file = self.state.opened_file.as_ref().unwrap();
                    if let Some((
                        line_no_jobs,
                        file_jobs,
                        points_of_interest,
                        visible_line_offsets,
                    )) = log_engine::recalculate_log_job(opened_file, &self.user_settings)
                    {
                        self.state.line_no_jobs = line_no_jobs;
                        self.state.log_jobs = file_jobs;
                        self.state.search_found = points_of_interest;
                        self.state.search_found_showing_index = 0;
                        self.state.search_found_last_shown_index = None;
                        self.state.visible_line_offsets = visible_line_offsets;
                    }
                }
            }
        }
    }

    fn show_line_numbers_scrollarea(
        &mut self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        visible_log_lines: usize,
        scroll_area_width_max: &mut f32,
        width_left_after_adding_line_numbers: &mut f32,
    ) {
        let mut opened_file_max_line_chars = 0;
        if let Some(opened_file) = &self.state.opened_file {
            opened_file_max_line_chars = opened_file.content_max_line_chars;
        }

        // Show the line numbers scroll area only if a file is opened.
        if opened_file_max_line_chars > 0 {
            egui::ScrollArea::vertical()
                .id_salt("line_numbers")
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .vertical_scroll_offset(self.state.vertical_scroll_offset)
                .animated(false)
                .scroll_source(self.scroll_sources_allowed)
                .show_rows(
                    ui,
                    self.user_settings.font.size,
                    visible_log_lines,
                    |ui, row_range| {
                        ui.set_min_height(ui.available_height());

                        ui.vertical(|ui| {
                            for row_index in row_range {
                                let line_wrapped_by = self.determine_wrapping(ctx, ui, row_index);

                                if let Some(job) = self
                                    .state
                                    .line_no_jobs
                                    .get(row_index - self.state.lines_wrapped)
                                {
                                    let mut job_cloned = job.clone();

                                    // Hack to add empty line numbers for wrapped lines, as
                                    // it's painful to do it properly with strange line spacings in single label.
                                    if line_wrapped_by > 0 {
                                        let text_format = egui::TextFormat {
                                            font_id: self.user_settings.font.clone(),
                                            ..Default::default()
                                        };

                                        job_cloned.append(
                                            "\n".repeat(line_wrapped_by).as_str(),
                                            0.0,
                                            text_format,
                                        );
                                    }

                                    let line_number_label = ui
                                        .add(
                                            egui::Label::new(job_cloned)
                                                .sense(egui::Sense::click()),
                                        )
                                        .on_hover_text("Click to add a comment")
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if line_number_label.clicked() {
                                        self.state.add_comment_request = Some(AddCommentRequest {
                                            line_no: self
                                                .state
                                                .visible_line_offsets
                                                .get_offset_for_visible_line(row_index + 1)
                                                + row_index
                                                + 1,
                                            ..Default::default()
                                        });
                                        self.state.add_comment_window_open = true;
                                    }

                                    if self.user_settings.comments_visible {
                                        let original_line_no = self
                                            .state
                                            .visible_line_offsets
                                            .get_offset_for_visible_line(row_index + 1)
                                            + row_index
                                            + 1;
                                        let comment_for_this_line_exists =
                                            self.state.opened_file.is_some() && {
                                                self.state
                                                    .opened_file
                                                    .as_ref()
                                                    .unwrap()
                                                    .log_comments
                                                    .contains_key(&original_line_no)
                                            };

                                        if comment_for_this_line_exists {
                                            // Account for comment line as well.
                                            // TODO: take into consideration wrapping of the comment too!
                                            let mut comment_job_dummy = LayoutJob::default();
                                            comment_job_dummy.append(
                                                "c",
                                                0.0,
                                                egui::TextFormat {
                                                    font_id: self.user_settings.font.clone(),
                                                    color: egui::Color32::LIGHT_GREEN,
                                                    italics: true,
                                                    ..Default::default()
                                                },
                                            );

                                            ui.horizontal(|ui| {
                                                let comment_label = ui
                                                    .add(
                                                        egui::Label::new(comment_job_dummy)
                                                            .sense(egui::Sense::click()),
                                                    )
                                                    .on_hover_text("Click to delete the comment")
                                                    .on_hover_cursor(egui::CursorIcon::Crosshair);
                                                if comment_label.clicked() {
                                                    if let Some(opened_file) =
                                                        &mut self.state.opened_file
                                                    {
                                                        opened_file
                                                            .log_comments
                                                            .remove(&original_line_no);
                                                    }
                                                }
                                            });
                                        }
                                    }
                                }
                            }

                            self.state.lines_wrapped = 0;
                        });

                        *width_left_after_adding_line_numbers = ui.available_width();
                    },
                );

            *scroll_area_width_max = if self.user_settings.wrap_text {
                *width_left_after_adding_line_numbers
            } else {
                (opened_file_max_line_chars as f32) * 8.0 + 50.0 // good enough
            };
        }
    }

    fn scroll_to_search_result(&mut self, ui: &egui::Ui, row_range: &std::ops::Range<usize>) {
        // If the search term is found, scroll to n-th occurence depending on the self.state.search_found_showing_index.
        if !self.state.search_found.is_empty() {
            let last_shown_different_or_init = (self.state.search_found_last_shown_index.is_none())
                || (self.state.search_found_last_shown_index.unwrap()
                    != self.state.search_found_showing_index);
            if last_shown_different_or_init {
                let poi = &self.state.search_found[self.state.search_found_showing_index];
                let line_of_interest = poi.line;

                // If we're not already showing the line, scroll to it.
                if row_range.start > line_of_interest - 1 || row_range.end <= line_of_interest - 1 {
                    let line_height = self.user_settings.font.size;
                    let current_top_line_offset = row_range.start as f32 * line_height;
                    let line_of_interest_offset = (line_of_interest as f32 - 1.0) * line_height;

                    // TODO: this delta should be adjusted to be more-or-less at the center of screen.
                    let delta: f32 = line_of_interest_offset - current_top_line_offset;

                    ui.scroll_with_delta(egui::vec2(0.0, -delta));
                } else {
                    // Scrolling completed.
                    self.state.search_found_last_shown_index =
                        Some(self.state.search_found_showing_index);
                }
            }
        }
    }

    fn show_comment_add_window(&mut self, ctx: &egui::Context) {
        if self.state.add_comment_request.is_none() {
            return;
        }

        egui::Window::new("Add Comment")
            .auto_sized()
            .collapsible(false)
            .open(&mut self.state.add_comment_window_open)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    let mut should_add_comment = false;

                    {
                        let comment_request = self.state.add_comment_request.as_mut().unwrap();

                        ui.label(format!(
                            "Adding comment to line #{}",
                            comment_request.line_no
                        ));

                        let comment_text_edit =
                            ui.text_edit_singleline(&mut comment_request.comment_text);
                        if comment_text_edit.lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        {
                            should_add_comment = true;
                        }
                        comment_text_edit.request_focus();
                    }

                    ui.horizontal(|ui| {
                        let comment_request = self.state.add_comment_request.as_mut().unwrap();

                        let button_add = ui.button("OK");
                        if button_add.clicked() || should_add_comment {
                            if !comment_request.comment_text.is_empty() {
                                if let Some(opened_file) = &mut self.state.opened_file {
                                    opened_file.log_comments.insert(
                                        comment_request.line_no,
                                        comment_request.comment_text.clone(),
                                    );
                                }

                                self.state.add_comment_request = None;
                                ui.close_kind(egui::UiKind::Window);
                            }
                        }

                        let button_cancel = ui.button("Cancel");
                        if button_cancel.clicked() {
                            self.state.add_comment_request = None;
                            ui.close_kind(egui::UiKind::Window);
                        }
                    });
                });
            });
    }
}

impl Default for LogalyzerGUI {
    fn default() -> Self {
        Self {
            user_settings: UserSettings::default(),
            user_settings_cached: UserSettings::default(),
            user_settings_staging: UserSettings::default(),
            state: LogalyzerState::default(),
            scroll_sources_allowed: scroll_area::ScrollSource {
                scroll_bar: true,
                drag: false,
                mouse_wheel: true,
            },
        }
    }
}

impl eframe::App for LogalyzerGUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let available_rect = ctx.available_rect();

        let bottom_panel_height = available_rect.height() * 0.2;
        let central_panel_height = available_rect.height() - bottom_panel_height;

        egui::TopBottomPanel::bottom("controls")
            .max_height(bottom_panel_height)
            .resizable(false)
            .show(ctx, |ui| {
                self.check_keyboard_shortcuts(ui);

                self.show_bottom_panel_first_row(ui);
                self.show_bottom_panel_search_and_filter(ui);
            });

        self.show_log_format_window(ctx);
        self.show_token_colors_panel(ctx);

        self.recalculate_logfile_display();

        let visible_log_lines = self.state.line_no_jobs.len();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_min_height(central_panel_height);

            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                let mut width_left_after_adding_line_numbers = ui.available_width();
                let mut scroll_area_width_max = ui.available_width();

                self.show_line_numbers_scrollarea(
                    ctx,
                    ui,
                    visible_log_lines,
                    &mut scroll_area_width_max,
                    &mut width_left_after_adding_line_numbers,
                );

                self.show_comment_add_window(ctx);

                let scroll_delta_keyboard = self.get_scroll_delta_based_on_keypress(
                    ctx,
                    ui,
                    central_panel_height,
                    width_left_after_adding_line_numbers,
                );

                let log_file_contents_scroll_area_resp = egui::ScrollArea::both()
                    .id_salt("log_file")
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysVisible)
                    .max_width(scroll_area_width_max)
                    .animated(false)
                    .scroll_source(self.scroll_sources_allowed)
                    .auto_shrink(false)
                    .show_rows(
                        ui,
                        self.user_settings.font.size,
                        visible_log_lines,
                        |ui, row_range| {
                            ui.set_min_height(ui.available_height());
                            ui.scroll_with_delta(scroll_delta_keyboard);

                            self.scroll_to_search_result(ui, &row_range);

                            let mut text_wrapping = TextWrapping::default();
                            if self.user_settings.wrap_text {
                                text_wrapping.break_anywhere = false;
                            }

                            text_wrapping.max_width = scroll_area_width_max;
                            ui.set_width(scroll_area_width_max);

                            ui.vertical(|ui| {
                                for row_index in row_range {
                                    if let Some(job) = self.state.log_jobs.get(row_index) {
                                        let mut job_cloned = job.clone();
                                        job_cloned.wrap = text_wrapping.clone();

                                        let log_line_resp = ui.add(
                                            egui::Label::new(job_cloned)
                                                .wrap_mode(egui::TextWrapMode::Wrap),
                                        );

                                        if log_line_resp.hovered() {
                                            log_line_resp.highlight();
                                        }

                                        if self.user_settings.comments_visible {
                                            if let Some(opened_file) = &self.state.opened_file {
                                                let original_line_no = self
                                                    .state
                                                    .visible_line_offsets
                                                    .get_offset_for_visible_line(row_index + 1)
                                                    + row_index
                                                    + 1;

                                                let comment_for_this_line =
                                                    opened_file.log_comments.get(&original_line_no);
                                                if let Some(comment_text) = comment_for_this_line {
                                                    let mut comment_job = LayoutJob::default();
                                                    comment_job.append(
                                                        format!("\t// {}", comment_text).as_str(),
                                                        0.0,
                                                        egui::TextFormat {
                                                            font_id: self
                                                                .user_settings
                                                                .font
                                                                .clone(),
                                                            color: egui::Color32::LIGHT_GREEN,
                                                            italics: true,
                                                            ..Default::default()
                                                        },
                                                    );
                                                    ui.horizontal(|ui| {
                                                        ui.add(egui::Label::new(comment_job));
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            });
                        },
                    );

                // Keep the line numbers scroll area and log file content scroll area synchronized while scrolling.
                self.state.vertical_scroll_offset =
                    log_file_contents_scroll_area_resp.state.offset.y;
                self.state.log_scroll_area_width =
                    log_file_contents_scroll_area_resp.content_size.x;
            });
        });
    }
}
