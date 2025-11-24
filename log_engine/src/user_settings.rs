use egui::{Color32, FontId};
use serde::{Deserialize, Serialize};
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

// HACK: just a struct that doesnt use egui types, for ser/des.
#[derive(Serialize, Deserialize)]
struct UserSettingsSerDes {
    pub wrap_text: bool,
    pub autoscroll: bool,
    pub search_term: String,
    pub search_match_case: bool,
    pub search_whole_word: bool,
    pub filter_term: String,
    pub filter_match_case: bool,
    pub filter_whole_word: bool,
    pub filter_negative: bool,
    pub log_format_pattern: String,
    pub log_format_pattern_coloring: Vec<(u8, u8, u8, u8)>, // RGBA
    pub token_colors: Vec<(String, (u8, u8, u8, u8))>,      // token_name, RGBA
    pub font_size: f32,
}

impl UserSettings {
    pub fn serialize(&self) -> Result<String, Box<dyn Error>> {
        let ser_des = UserSettingsSerDes {
            wrap_text: self.wrap_text,
            autoscroll: self.autoscroll,
            search_term: self.search_term.clone(),
            search_match_case: self.search_match_case,
            search_whole_word: self.search_whole_word,
            filter_term: self.filter_term.clone(),
            filter_match_case: self.filter_match_case,
            filter_whole_word: self.filter_whole_word,
            filter_negative: self.filter_negative,
            log_format_pattern: self.log_format.pattern.clone(),
            log_format_pattern_coloring: self
                .log_format
                .pattern_coloring
                .iter()
                .map(|c| (c.r(), c.g(), c.b(), c.a()))
                .collect(),
            token_colors: self
                .token_colors
                .iter()
                .map(|(name, color)| (name.clone(), (color.r(), color.g(), color.b(), color.a())))
                .collect(),
            font_size: self.font.size,
        };

        let serialized = serde_json::to_string_pretty(&ser_des)?;

        Ok(serialized)
    }

    pub fn deserialize(str: &String) -> Result<UserSettings, Box<dyn Error>> {
        let ser_des: UserSettingsSerDes = serde_json::from_str(str)?;

        let log_format = LogFormat {
            pattern: ser_des.log_format_pattern,
            pattern_coloring: ser_des
                .log_format_pattern_coloring
                .iter()
                .map(|(r, g, b, a)| Color32::from_rgba_unmultiplied(*r, *g, *b, *a))
                .collect(),
        };

        let token_colors = ser_des
            .token_colors
            .iter()
            .map(|(name, (r, g, b, a))| {
                (
                    name.clone(),
                    Color32::from_rgba_unmultiplied(*r, *g, *b, *a),
                )
            })
            .collect();

        Ok(UserSettings {
            wrap_text: ser_des.wrap_text,
            autoscroll: ser_des.autoscroll,
            search_term: ser_des.search_term,
            search_match_case: ser_des.search_match_case,
            search_whole_word: ser_des.search_whole_word,
            filter_term: ser_des.filter_term,
            filter_match_case: ser_des.filter_match_case,
            filter_whole_word: ser_des.filter_whole_word,
            filter_negative: ser_des.filter_negative,
            file_path: String::new(),
            log_format,
            token_colors,
            font: FontId::monospace(ser_des.font_size),
        })
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
