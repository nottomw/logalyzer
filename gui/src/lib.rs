use core::f32;

use clap::Parser;
use eframe::egui;
use egui::containers::scroll_area::ScrollBarVisibility;
use egui::text::{LayoutJob, TextWrapping};
use egui::{Vec2, scroll_area};
use log_engine::OpenedFileMetadata;
use log_engine::user_settings::UserSettings;
use std::path::Path;

pub fn run_gui() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        ..Default::default()
    };

    let app_name = format!("Logalyzer ({})", env!("CARGO_PKG_VERSION"));

    let run_result = eframe::run_native(
        &app_name,
        options,
        Box::new(|_cc| Ok(Box::new(LogalyzerGUI::new()) as Box<dyn eframe::App>)),
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
    win_histogram_open: bool,
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
            win_histogram_open: false,
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

#[derive(Parser)]
#[command(name = "logalyzer", version, about = "Logalyzer log analysis tool.", long_about = None)]
struct LogalyzerArgs {
    /// Path to the log file to open.
    #[arg(short, long, long = "file")]
    file_path: Option<String>,
    /// Path to the configuration file to load.
    #[arg(short, long, long = "config")]
    config_path: Option<String>,
}

impl LogalyzerGUI {
    fn new() -> Self {
        let mut new_self = Self::default();

        let args = LogalyzerArgs::parse();
        if let Some(file_path) = args.file_path {
            if !Path::new(&file_path).exists() {
                println!("Specified log file does not exist: {}", file_path);
            } else {
                new_self.user_settings.file_path = file_path;
            }
        }

        if let Some(config_path_str) = args.config_path {
            if !Path::new(&config_path_str).exists() {
                println!("Specified config file does not exist: {}", config_path_str);
            } else {
                let config_path = Path::new(&config_path_str);
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

            // TODO: append file / prepend file options?

            // Maybe later ;)
            // let button_stream = ui.button("Open Stream");
            // if button_stream.clicked() {
            //     println!("not implemented");
            // }

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
                self.state.win_histogram_open = !self.state.win_histogram_open;
            }

            // let button_stats = ui.add_enabled(file_opened, egui::Button::new("Stats"));
            // if button_stats.clicked() {
            //     println!("not implemented");
            // }

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

            // Maybe later ;)
            // ui.add_enabled(
            //     false, // This should be on only if a stream is opened.
            //     egui::Checkbox::new(&mut self.user_settings.autoscroll, "Autoscroll"),
            // );

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

                if textedit_search.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    // On enter in search input move to next result.
                    if !self.state.search_found.is_empty() {
                        self.state.search_found_showing_index =
                            (self.state.search_found_showing_index + 1)
                                % self.state.search_found.len();
                    }

                    // Keep the focus.
                    textedit_search.request_focus();
                }

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
                ui.checkbox(&mut self.user_settings.filter_negative, "Negative")
                    .on_hover_text("Show lines that DO NOT match the filter term.");
                ui.checkbox(&mut self.user_settings.filter_extended, "Extended")
                    .on_hover_text(
                        "Enable simple extended filtering with either only && clauses or only || clauses.\nExample: \"error && failed && stack trace\"\nExample: \"error || warning || info\"",
                    );
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

    // Returns (line_range_start, line_range_end, number_of_entries)
    fn histogram_find_matches(
        &self,
        number_of_bars: usize,
        match_case: bool,
    ) -> Vec<(usize, usize, usize)> {
        let mut matches = Vec::new();

        if self.user_settings_staging.histogram_search_term.is_empty() {
            return matches;
        }

        if let Some(opened_file) = &self.state.opened_file {
            let line_range_size = ((opened_file.content_line_count as f64)
                / (number_of_bars as f64))
                .floor() as usize;

            for bar_index in 0..number_of_bars {
                let line_range_start = bar_index * line_range_size;
                let mut line_range_end = (bar_index + 1) * line_range_size;
                if bar_index == number_of_bars - 1 {
                    line_range_end = opened_file.content_line_count;
                }

                // Grab all lines from the range.
                let lines_in_range = opened_file
                    .content
                    .lines()
                    .skip(line_range_start)
                    .take(line_range_end - line_range_start)
                    .map(|line| {
                        if !match_case {
                            line.to_lowercase()
                        } else {
                            line.to_string()
                        }
                    });

                let search_term = if !match_case {
                    self.user_settings_staging
                        .histogram_search_term
                        .to_lowercase()
                } else {
                    self.user_settings_staging.histogram_search_term.clone()
                };

                let matches_in_range = lines_in_range
                    .filter(|line| line.contains(&search_term))
                    .count();

                matches.push((line_range_start + 1, line_range_end, matches_in_range));
            }
        }

        matches
    }

    fn show_histogram_window(&mut self, ctx: &egui::Context) {
        let mut histogram_matches: Vec<(usize, usize, usize)> = Vec::new();
        let number_of_bars = 10;

        if !self.user_settings_staging.histogram_search_term.is_empty() {
            histogram_matches = self.histogram_find_matches(
                number_of_bars,
                self.user_settings_staging.histogram_match_case,
            );
        }

        egui::Window::new("Histogram")
            .auto_sized()
            .collapsible(false)
            .open(&mut self.state.win_histogram_open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Histogram term:");
                        ui.add_sized(
                            [300.0, 20.0],
                            egui::TextEdit::singleline(
                                &mut self.user_settings_staging.histogram_search_term,
                            )
                            .id_salt("histogram_search_input"),
                        );

                        ui.checkbox(
                            &mut self.user_settings_staging.histogram_match_case,
                            "Match case",
                        );
                    });

                    let mut highest_count_index: usize = 0;
                    let mut lowest_count_index: isize = -1;

                    for (i, (_, _, count)) in histogram_matches.iter().enumerate() {
                        if histogram_matches[highest_count_index].2 < *count {
                            highest_count_index = i;
                        }

                        if (lowest_count_index == -1
                            || histogram_matches[lowest_count_index as usize].2 > *count)
                            && (*count > 0)
                        {
                            lowest_count_index = i as isize;
                        }
                    }

                    if lowest_count_index == -1 {
                        lowest_count_index = 0; // just to have some value
                    }

                    ui.add_space(5.0);

                    egui::Grid::new("histogram_grid")
                        .num_columns(3)
                        .show(ui, |ui| {
                            let mut range_index = 0;

                            ui.label("Range");
                            ui.label("Count");
                            ui.label("");
                            ui.end_row();

                            if histogram_matches.len() != 0 {
                                for (hist_start, hist_end, hist_count) in histogram_matches.iter() {
                                    ui.label(format!("{} - {}", hist_start, hist_end));

                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(format!("{}", hist_count));
                                        },
                                    );

                                    let bar_height = self.user_settings.font.size;
                                    let bar_width_max = 350.0;
                                    let bar_width = if histogram_matches[highest_count_index].2 > 0
                                    {
                                        ((*hist_count as f32)
                                            / (histogram_matches[highest_count_index].2 as f32))
                                            * bar_width_max
                                    } else {
                                        0.0
                                    };
                                    let bar_color = if range_index == highest_count_index {
                                        egui::Color32::LIGHT_RED
                                    } else if range_index == lowest_count_index as usize {
                                        egui::Color32::LIGHT_GREEN
                                    } else {
                                        egui::Color32::LIGHT_BLUE
                                    };

                                    let (response, painter) = ui.allocate_painter(
                                        Vec2::new(bar_width, bar_height),
                                        egui::Sense::empty(),
                                    );

                                    let rect = response.rect;
                                    painter.rect_filled(rect, 0.0, bar_color);

                                    ui.end_row();

                                    range_index += 1;
                                }
                            } else {
                                for _ in 0..number_of_bars {
                                    // Draw the table anyway with empty fields.
                                    ui.label("-");
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label("0");
                                        },
                                    );

                                    ui.label("");
                                    ui.end_row();
                                }
                            }
                        });
                });
            });
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
        if !self.state.search_found.is_empty() {
            let last_shown_different_or_init = (self.state.search_found_last_shown_index.is_none())
                || (self.state.search_found_last_shown_index.unwrap()
                    != self.state.search_found_showing_index);
            if last_shown_different_or_init {
                let poi = &self.state.search_found[self.state.search_found_showing_index];
                let line_of_interest = poi.line;

                let line_before_current_range = line_of_interest - 1 < row_range.start;
                let line_after_current_range = line_of_interest - 1 >= row_range.end;

                if line_before_current_range {
                    // Scrolling up.

                    let line_diff = row_range.start as isize - (line_of_interest as isize - 1);
                    let delta = (line_diff as f32) * self.user_settings.font.size;

                    ui.scroll_with_delta(egui::vec2(0.0, delta));
                } else if line_after_current_range {
                    // Scrolling down.

                    let line_diff = (line_of_interest as isize - 1) - row_range.end as isize + 1;
                    let delta = (line_diff as f32) * self.user_settings.font.size;

                    ui.scroll_with_delta(egui::vec2(0.0, -delta));
                } else {
                    // Reached the requested range, but do a last effort scroll to try and align
                    // the line more to center of screen.

                    let range_center = (row_range.start + row_range.end) / 2;
                    let line_diff = line_of_interest as isize - 1 - range_center as isize;
                    let delta = (line_diff as f32) * self.user_settings.font.size;

                    ui.scroll_with_delta(egui::vec2(0.0, -delta));

                    // Mark scrolling as completed.
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
        self.show_histogram_window(ctx);

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
                    .animated(false)
                    .scroll_source(self.scroll_sources_allowed)
                    .auto_shrink(false)
                    .show_rows(
                        ui,
                        self.user_settings.font.size,
                        visible_log_lines,
                        |ui, row_range| {
                            ui.take_available_space();
                            ui.set_min_height(ui.available_height());
                            ui.scroll_with_delta(scroll_delta_keyboard);

                            self.scroll_to_search_result(ui, &row_range);

                            let mut text_wrapping = TextWrapping::default();
                            if self.user_settings.wrap_text {
                                text_wrapping.break_anywhere = false;
                            } else {
                                ui.set_width(scroll_area_width_max);
                            }

                            text_wrapping.max_width = scroll_area_width_max;

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
