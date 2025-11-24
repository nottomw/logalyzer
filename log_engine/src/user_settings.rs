use egui::{Color32, FontId};
use std::error::Error;

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

impl UserSettings {
    pub fn serialize(&self) -> Result<String, Box<dyn Error>> {
        Ok(String::new())
    }

    pub fn deserialize(&self) -> Result<UserSettings, Box<dyn Error>> {
        Ok(UserSettings::default())
    }
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
            token_colors: Vec::with_capacity(25),
            font: FontId::monospace(12.0),
        };

        // Initialize the colors in token_colors to some default values.
        for i in 0..new_instance.token_colors.capacity() {
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
