use egui::{
    Color32, FontId,
    text::{LayoutJob, TextFormat},
};

#[derive(PartialEq, Clone, Default)]
pub struct LogFormat {
    pub pattern: String, // matching regex (i.e. "^\[[0-9]*\.[0.9]*\] .*$")
    pub pattern_coloring: Vec<egui::Color32>, // coloring for each regex group (i.e. "yellow,green,nocolor")
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

pub fn load_file(path: &String) -> (LayoutJob, Option<OpenedFileMetadata>) {
    println!("Loading file: {}", path);
    let mut job = LayoutJob::default();

    let read_result = std::fs::read_to_string(&path);
    if read_result.is_err() {
        job.append(
            &format!("Failed to read file: {}\n", path),
            0.0,
            TextFormat::default(),
        );
        return (job, None);
    }

    let file_content = read_result.unwrap();

    let text_format = TextFormat {
        font_id: FontId::monospace(12.0),
        ..Default::default()
    };

    job.append(&file_content, 0.0, text_format);

    let mut opened_file_meta = OpenedFileMetadata::default();
    opened_file_meta.path = path.clone();
    opened_file_meta.content = file_content.clone();
    opened_file_meta.content_max_line_chars = file_content
        .lines()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    opened_file_meta.content_line_count = file_content.lines().count();

    (job, Some(opened_file_meta))
}

fn color_to_text_format(color_name: egui::Color32) -> TextFormat {
    let mut text_format = TextFormat::default();
    text_format.font_id = FontId::monospace(12.0);

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

pub fn recalculate_log_job(
    opened_file: &OpenedFileMetadata,
    user_settings: &UserSettings,
) -> Option<LayoutJob> {
    let mut job = LayoutJob::default();

    let mut feature_log_format = false;
    let mut feature_log_format_regex: regex::Regex = regex::Regex::new("").unwrap();

    if !user_settings.log_format.pattern.is_empty()
        && !user_settings.log_format.pattern_coloring.is_empty()
    {
        let log_format = &user_settings.log_format;

        let feature_log_format_regex_result = regex::Regex::new(&log_format.pattern);
        if feature_log_format_regex_result.is_err() {
            println!(
                "Invalid regex pattern for log formatting: {}",
                log_format.pattern
            );
        } else {
            feature_log_format = true;
            feature_log_format_regex = feature_log_format_regex_result.unwrap();
        }
    }

    for line in opened_file.content.lines() {
        if feature_log_format {
            let line_matched_groups = feature_log_format_regex.captures(line);
            if line_matched_groups.is_none() {
                let text_format = TextFormat {
                    font_id: FontId::monospace(12.0),
                    ..Default::default()
                };

                job.append(&format!("{}\n", line), 0.0, text_format);
                continue;
            }

            let line_matched_groups = line_matched_groups.unwrap();

            // Verify the number of captures match the number of coloring pattern.
            let actual_group_count = line_matched_groups.len() - 1; // 1 for original line
            if actual_group_count != user_settings.log_format.pattern_coloring.len() {
                let text_format = TextFormat {
                    font_id: FontId::monospace(12.0),
                    ..Default::default()
                };

                job.append(&format!("{}\n", line), 0.0, text_format);
                continue;
            }

            // Do the actual coloring.
            for (i, group) in line_matched_groups.iter().enumerate() {
                // Skip first group which is always a full line.
                if group.is_none() || i == 0 {
                    continue;
                }

                let group_str = group.unwrap().as_str();
                let group_str_coloring = user_settings.log_format.pattern_coloring[i - 1];
                let text_format = color_to_text_format(group_str_coloring);

                // If this is the last matching group, append a newline.
                if i == line_matched_groups.len() - 1 {
                    job.append(&format!("{}\n", group_str), 0.0, text_format);
                    continue;
                }

                job.append(group_str, 0.0, text_format);
            }
        } else {
            let text_format = TextFormat {
                font_id: FontId::monospace(12.0),
                ..Default::default()
            };

            job.append(&format!("{}\n", line), 0.0, text_format);
        }
    }

    Some(job)
}
