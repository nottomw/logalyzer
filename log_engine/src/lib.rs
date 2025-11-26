use egui::{
    FontId,
    text::{LayoutJob, TextFormat},
};

use std::error::Error;

pub mod line_handlers;
pub mod user_settings;

use crate::line_handlers::*;
use crate::user_settings::*;

#[derive(Clone)]
pub struct PointOfInterest {
    pub line: usize,
    pub line_part_index: usize,
    pub line_offset: usize,
    pub line_point_size: usize,
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

    let welcome_message = "Welcome to Logalyzer.\n\n\
    Please select a log file or a stream to open.\n\
    Please use the settings panel to configure log formatting and highlighting options.\n\
    You can use WASD to navigate quickly through the log file.\n";

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

fn make_line_handlers(user_settings: &UserSettings) -> Vec<Box<dyn LineHandler>> {
    let mut handlers: Vec<Box<dyn LineHandler>> = Vec::new();

    // The filter should be first, so we're not applying other handlers to lines that will be invisible anyway.
    let filter_line_handler = FilterLineHandler::new(user_settings);
    if let Some(handler) = filter_line_handler {
        if handler.is_active() {
            handlers.push(Box::from(handler));
        }
    }

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

    let search_line_handler = SearchLineHandler::new(user_settings);
    if let Some(handler) = search_line_handler {
        if handler.is_active() {
            handlers.push(Box::from(handler));
        }
    }

    handlers
}

// Returns a tuple of (line number layout jobs, log lines layout jobs)
// TODO: this should not return anything related to LayoutJob, Vec<Vec<String, TextFormat>> would be better.
pub fn recalculate_log_job(
    opened_file: &OpenedFileMetadata,
    user_settings: &UserSettings,
) -> Option<(Vec<LayoutJob>, Vec<LayoutJob>, Vec<PointOfInterest>)> {
    let mut jobs_log: Vec<LayoutJob> = Vec::new();
    let mut jobs_line_numbers: Vec<LayoutJob> = Vec::new();
    let mut points_of_interest: Vec<PointOfInterest> = Vec::new();

    let mut handlers = make_line_handlers(user_settings);

    let mut lines_visible = 0;

    let default_text_format = TextFormat {
        font_id: user_settings.font.clone(),
        ..Default::default()
    };

    for line in opened_file.content.lines() {
        let mut single_line_job = LayoutJob::default();

        if !handlers.is_empty() {
            let mut line_parts: Vec<(String, TextFormat)> =
                vec![(line.to_string(), default_text_format.clone())];

            for handler in &mut handlers {
                handler.process_line(&mut line_parts);

                // This should ideally be fixed, as we're uncovering here the line handler type.
                if handler.handler_type() == LineHandlerType::Search {
                    let mut points_of_interest_in_line = handler.points_of_interest();
                    if points_of_interest_in_line.is_empty() {
                        continue;
                    }

                    // Set line number in each point of interest, as the line handler don't know it.
                    for poi in &mut points_of_interest_in_line {
                        poi.line = lines_visible + 1;
                    }

                    println!("Found term in line {}", lines_visible + 1);

                    points_of_interest.append(&mut points_of_interest_in_line);
                }
            }

            for (part_str, part_format) in line_parts {
                single_line_job.append(&part_str, 0.0, part_format);
            }
        } else {
            single_line_job.append(line, 0.0, default_text_format.clone());
        }

        if !single_line_job.is_empty() {
            lines_visible += 1;
            jobs_log.push(single_line_job);
        }
    }

    // TODO: show also original lines i.e. in case of filtering
    for line_no in 1..=lines_visible {
        let mut single_line_no_job = LayoutJob::default();

        single_line_no_job.append(&format!("{}", line_no), 0.0, default_text_format.clone());

        jobs_line_numbers.push(single_line_no_job);
    }

    Some((jobs_line_numbers, jobs_log, points_of_interest))
}

pub fn configuration_save(file_path: &std::path::Path, user_settings: &UserSettings) {
    println!(
        "Trying to save configuration to: {}",
        file_path.to_string_lossy()
    );

    let serialized = user_settings.serialize();
    if let Err(e) = serialized {
        println!("Error serializing configuration: {}", e);
        return;
    }

    let write_result = std::fs::write(file_path, serialized.unwrap());
    if let Err(e) = write_result {
        println!("Error writing configuration to file: {}", e);
        return;
    }

    println!("Configuration saved successfully.");
}

pub fn configuration_load(file_path: &std::path::Path) -> Result<UserSettings, Box<dyn Error>> {
    println!(
        "Trying to load configuration from: {}",
        file_path.to_string_lossy()
    );

    let read_result = std::fs::read_to_string(file_path);
    if let Err(e) = read_result {
        println!("Error reading configuration file: {}", e);
        return Err(Box::new(e));
    }

    let deserialized = UserSettings::deserialize(&read_result.unwrap());
    if let Err(e) = deserialized {
        println!("Error deserializing configuration: {}", e);
        return Err(e);
    }

    println!("Configuration loaded successfully.");

    Ok(deserialized.unwrap())
}
