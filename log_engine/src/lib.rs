use egui::{
    FontId,
    text::{LayoutJob, TextFormat},
};

mod line_handlers;
pub mod user_settings;

use crate::line_handlers::*;
use crate::user_settings::*;

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
    let file_content_max_line_chars = file_content
        .lines()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    let file_content_line_count = file_content.lines().count();

    let mut opened_file_meta = OpenedFileMetadata::default();
    opened_file_meta.path = path.clone();
    opened_file_meta.content = file_content;
    opened_file_meta.content_max_line_chars = file_content_max_line_chars;
    opened_file_meta.content_line_count = file_content_line_count;

    Some(opened_file_meta)
}

pub fn recalculate_log_job(
    opened_file: &OpenedFileMetadata,
    user_settings: &UserSettings,
) -> Option<LayoutJob> {
    let mut job = LayoutJob::default();

    let mut handlers: Vec<Box<dyn LineHandler>> = Vec::new();

    let log_format_line_handler = LogFormatLineHandler::new(user_settings);
    if let Some(handler) = log_format_line_handler {
        if handler.is_active() {
            handlers.push(Box::from(handler));
        }
    }

    let token_hilight_line_handler = TokenHilightLineHandler::new(user_settings);
    if let Some(handler) = token_hilight_line_handler {
        if handler.is_active() {
            handlers.push(Box::from(handler));
        }
    }

    for line in opened_file.content.lines() {
        if !handlers.is_empty() {
            let mut line_parts: Vec<(String, TextFormat)> = vec![(
                line.to_string(),
                TextFormat {
                    font_id: user_settings.font.clone(),
                    ..Default::default()
                },
            )];

            for handler in &handlers {
                handler.process_line(&mut line_parts);
            }

            // Add newline to the last line part if it's not already there.
            let line_parts_len = line_parts.len();
            let ends_with_newline = line_parts[line_parts_len - 1].0.ends_with("\n");
            if !ends_with_newline {
                line_parts[line_parts_len - 1].0 += "\n";
            }

            for (part_str, part_format) in line_parts {
                job.append(&part_str, 0.0, part_format);
            }
        } else {
            println!("DBG: no line handlers append");
            job.append(
                &format!("{}\n", line),
                0.0,
                TextFormat {
                    font_id: user_settings.font.clone(),
                    ..Default::default()
                },
            );
        }
    }

    Some(job)
}
