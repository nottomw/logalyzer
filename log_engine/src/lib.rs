use egui::{
    Color32, FontId,
    text::{LayoutJob, TextFormat},
};

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

pub fn load_file(path: String) -> (LayoutJob, Option<OpenedFileMetadata>) {
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
