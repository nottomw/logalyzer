use egui::{
    Color32, FontId,
    epaint::tessellator::path,
    text::{LayoutJob, TextFormat},
};

mod line_handlers;
mod user_settings;

#[derive(PartialEq, Clone, Default)]
pub struct LogFormat {
    pub pattern: String, // matching regex (i.e. "^\[[0-9]*\.[0.9]*\] .*$")
    pub pattern_coloring: Vec<egui::Color32>,
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
    let token_hilight_line_handler = TokenHilightLineHandler::new(user_settings);

    let handlers: Vec<Option<LineHandler>> =
        vec![log_format_line_handler, token_hilight_line_handler];

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
