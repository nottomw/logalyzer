use egui::{
    Color32, FontId,
    epaint::tessellator::path,
    text::{LayoutJob, TextFormat},
};

#[derive(PartialEq, Clone, Default)]
pub struct LogFormat {
    pub pattern: String, // matching regex (i.e. "^\[[0-9]*\.[0.9]*\] .*$")
    pub pattern_coloring: Vec<egui::Color32>,
}

#[derive(PartialEq, Clone)]
pub struct UserSettings {
    pub wrap_text: bool,
    pub autoscroll: bool,
    pub search_term: String,
    pub search_match_case: bool,
    pub search_whole_word: bool,
    pub filter_term: String,
    pub filter_match_case: bool,
    pub filter_whole_word: bool,
    pub filter_negative: bool,
    pub file_path: String,
    pub log_format: LogFormat,
    pub token_colors: Vec<(String, Color32)>,
    pub font: FontId,
}

impl Default for UserSettings {
    fn default() -> Self {
        let mut new_instance = UserSettings {
            wrap_text: false,
            autoscroll: false,
            search_term: String::new(),
            search_match_case: false,
            search_whole_word: false,
            filter_term: String::new(),
            filter_match_case: false,
            filter_whole_word: false,
            filter_negative: false,
            file_path: String::new(),
            log_format: LogFormat::default(),
            token_colors: Vec::with_capacity(20),
            font: FontId::monospace(12.0),
        };

        // Initialize the colors in token_colors to some default values.
        for i in 0..20 {
            let color = Color32::from_rgb(
                (i * 12 % 256) as u8,
                (i * 34 % 256) as u8,
                (i * 56 % 256) as u8,
            );

            new_instance.token_colors.push((String::new(), color));
        }

        new_instance
    }
}

pub struct OpenedFileMetadata {
    pub path: String,
    pub content: String,
    pub content_max_line_chars: usize,
    pub content_line_count: usize,
}

impl Default for OpenedFileMetadata {
    fn default() -> Self {
        Self {
            path: String::new(),
            content: String::new(),
            content_max_line_chars: 0,
            content_line_count: 0,
        }
    }
}

pub fn default_log_content() -> LayoutJob {
    let mut job = LayoutJob::default();

    let welcome_message = "Please select a log file or a stream to open.";

    job.append(
        welcome_message,
        0.0,
        TextFormat {
            font_id: FontId::monospace(12.0),
            ..Default::default()
        },
    );

    job
}

pub fn load_file(user_settings: &UserSettings) -> Option<OpenedFileMetadata> {
    let path = user_settings.file_path.clone();
    println!("Loading file: {}", path);

    let read_result = std::fs::read_to_string(&path);
    if read_result.is_err() {
        println!(
            "Failed to read file: {}, error: {}",
            path,
            read_result.err().unwrap()
        );
        return None;
    }

    let file_content = read_result.unwrap();

    let mut opened_file_meta = OpenedFileMetadata::default();
    opened_file_meta.path = path.clone();
    opened_file_meta.content = file_content;
    opened_file_meta.content_max_line_chars = file_content
        .lines()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    opened_file_meta.content_line_count = file_content.lines().count();

    Some(opened_file_meta)
}

fn color_to_text_format(color_name: egui::Color32, font: FontId) -> TextFormat {
    let mut text_format = TextFormat::default();
    text_format.font_id = font;

    text_format.background = color_name;

    // Ensure the text color is visible on the background.
    // If it's bright, make the color black, else white.
    text_format.color =
        if (color_name.r() as u32 + color_name.g() as u32 + color_name.b() as u32) / 3 > 128 {
            Color32::BLACK
        } else {
            Color32::WHITE
        };

    text_format
}

trait LineHandler {
    fn is_active(&self) -> bool;
    fn process_line(&self, line: &mut Vec<(String, TextFormat)>);
}

struct LogFormatLineHandler {
    compiled_log_format_regex: regex::Regex,
}

impl LogFormatLineHandler {
    pub fn new(user_settings: &UserSettings) -> Option<Self> {
        if user_settings.log_format.pattern.is_empty()
            || user_settings.log_format.pattern_coloring.is_empty()
        {
            return None;
        }

        let compiled_regex = regex::Regex::new(&user_settings.log_format.pattern);
        if compiled_regex.is_err() {
            return None;
        }

        Some(Self {
            compiled_log_format_regex: regex::Regex::new(&user_settings.log_format.pattern)
                .unwrap(),
        })
    }
}

impl LineHandler for LogFormatLineHandler {
    fn is_active(&self) -> bool {
        if compiled_log_format_regex.is_valid() {
            return true;
        }

        return false;
    }

    fn process_line(&self, line: &mut Vec<(String, TextFormat)>) {
        // Log format works only on full lines.
        assert!(line.len() == 1);

        let line_full = &line[0].0;

        // If nothing matched do nothing.
        let line_matched_groups_res = feature_log_format_regex.captures(line_full);
        if line_matched_groups_res.is_none() {
            return;
        }

        let line_matched_groups = line_matched_groups_res.unwrap();

        // Verify the number of captures match the number of coloring pattern.
        let actual_group_count = line_matched_groups.len() - 1; // 1 for original line
        if actual_group_count != user_settings.log_format.pattern_coloring.len() {
            return;
        }

        // Do the actual coloring.
        let mut line_result: Vec<(String, TextFormat)> = Vec::new();

        for (i, group) in line_matched_groups.iter().enumerate() {
            // Skip first group which is always a full match.
            if group.is_none() || i == 0 {
                return;
            }

            let group_str = group.unwrap().as_str();
            let group_str_coloring = user_settings.log_format.pattern_coloring[i - 1];
            let text_format = color_to_text_format(group_str_coloring, user_settings.font);

            // If this is the last matching group, append a newline.
            if i == line_matched_groups.len() - 1 {
                line_result.push((format!("{}\n", group_str), text_format));
                return;
            }

            line_result.push((group_str.to_string(), text_format));
        }

        *line = line_result;
    }
}

pub fn recalculate_log_job(
    opened_file: &OpenedFileMetadata,
    user_settings: &UserSettings,
) -> Option<LayoutJob> {
    let text_format_default = TextFormat {
        font_id: user_settings.font,
        ..Default::default()
    };

    let mut job = LayoutJob::default();
    let log_format_line_handler = LogFormatLineHandler::new(user_settings);

    let handlers: Vec<Option<LineHandler>> = vec![log_format_line_handler];

    for line in opened_file.content.lines() {
        if !handlers.is_empty() {
            let mut line_parts: Vec<(String, TextFormat)> =
                vec![(line.to_string(), text_format_default)];

            for handler_opt in &handlers {
                if handler_opt.is_none() {
                    continue;
                }

                let handler = handler_opt.as_ref().unwrap();
                if !handler.is_active() {
                    continue;
                }

                handler.process_line(&mut line_parts);
            }

            for (part_str, part_format) in line_parts {
                job.append(&part_str, 0.0, part_format);
            }
        } else {
            job.append(&format!("{}\n", line), 0.0, text_format_default);
        }
    }

    Some(job)
}
