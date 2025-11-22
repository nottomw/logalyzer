use egui::{
    Color32, FontId,
    text::{LayoutJob, TextFormat},
};

#[derive(PartialEq, Clone)]
pub struct LogFormat {
    pub pattern: String,          // matching regex (i.e. "^\[[0-9]*\.[0.9]*\] .*$")
    pub pattern_coloring: String, // coloring for each regex group (i.e. "yellow,green,nocolor")
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
            wrap_text: true,
            autoscroll: false,
            search_term: String::new(),
            search_match_case: false,
            search_whole_word: false,
            filter_term: String::new(),
            filter_match_case: false,
            filter_whole_word: false,
            filter_negative: false,
            file_path: String::new(),
            log_format: LogFormat {
                // TODO: For now a hardcoded pattern for tests.
                // Line example: [    0.000000] Linux version 6.8.0-57-generic (buildd@lcy02-amd64-040) (x86_64-linux-gnu-
                pattern: String::from(r"^(\[\s*[0-9]*)(\.)([0-9]*\])(\s.*)$"),
                pattern_coloring: String::from("yellow,nocolor,green,nocolor"),
            },
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
    pub content: String,
    pub content_max_line_chars: usize,
    pub content_line_count: usize,
}

impl Default for OpenedFileMetadata {
    fn default() -> Self {
        Self {
            content: String::new(),
            content_max_line_chars: 0,
            content_line_count: 0,
        }
    }
}

pub fn default_log_content() -> LayoutJob {
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
    opened_file_meta.content = file_content.clone();
    opened_file_meta.content_max_line_chars = file_content
        .lines()
        .map(|line| line.len())
        .max()
        .unwrap_or(0);
    opened_file_meta.content_line_count = file_content.lines().count();

    (job, Some(opened_file_meta))
}

fn color_name_to_text_format(color_name: &str) -> TextFormat {
    let mut text_format = TextFormat::default();
    text_format.font_id = FontId::monospace(12.0);

    match color_name {
        "red" => {
            text_format.background = Color32::RED;
            text_format.color = Color32::BLACK;
        }
        "green" => {
            text_format.background = Color32::GREEN;
            text_format.color = Color32::BLACK;
        }
        "yellow" => {
            text_format.background = Color32::YELLOW;
            text_format.color = Color32::BLACK;
        }
        "nocolor" => {} // No background color
        _ => {}         // Unknown color, keep default
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
    let mut feature_log_format_coloring_pattern_split: Vec<&str> = Vec::new();

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
            feature_log_format_coloring_pattern_split = log_format
                .pattern_coloring
                .split(',')
                .collect::<Vec<&str>>();
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
            if actual_group_count != feature_log_format_coloring_pattern_split.len() {
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
                let coloring_str = feature_log_format_coloring_pattern_split[i - 1];
                let text_format = color_name_to_text_format(coloring_str);

                // If this is the last matching group, append a newline.
                if i == line_matched_groups.len() - 1 {
                    job.append(&format!("{}\n", group_str), 0.0, text_format);
                    continue;
                }

                job.append(group_str, 0.0, text_format);
            }
        }
    }

    Some(job)
}
