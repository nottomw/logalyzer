use egui::text::TextFormat;
use egui::{Color32, FontId};

use crate::PointOfInterest;
use crate::linevec::*;
use crate::user_settings::UserSettings;

#[derive(PartialEq)]
pub enum LineHandlerType {
    LogFormat,
    TokenHilight,
    Filter,
    Search,
}

pub trait LineHandler {
    fn handler_type(&self) -> LineHandlerType;
    fn is_active(&self) -> bool;
    fn process_line(&mut self, line: &mut LineVec);
    fn points_of_interest(&self) -> Vec<PointOfInterest>;
}

fn calculate_text_color_from_background_color(color_background: egui::Color32) -> egui::Color32 {
    let color = if (color_background.r() as u32
        + color_background.g() as u32
        + color_background.b() as u32)
        / 3
        > 128
    {
        Color32::BLACK
    } else {
        Color32::WHITE
    };

    color
}

fn color_to_text_format_with_textcolor(
    color_background: egui::Color32,
    color_text: egui::Color32,
    font: FontId,
) -> TextFormat {
    let text_format = TextFormat {
        font_id: font,
        background: color_background,
        color: color_text,
        ..Default::default()
    };

    text_format
}

pub struct LogFormatLineHandler {
    compiled_log_format_regex: regex::Regex,
    pattern_coloring: Vec<Color32>,
    pattern_coloring_text: Vec<Color32>,
    pattern_coloring_text_use_original: Vec<bool>,
    default_font: FontId,
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
            compiled_log_format_regex: compiled_regex.unwrap(),
            pattern_coloring: user_settings.log_format.pattern_coloring.clone(),
            default_font: user_settings.font.clone(),
            pattern_coloring_text: user_settings.log_format.pattern_coloring_text.clone(),
            pattern_coloring_text_use_original: user_settings
                .log_format
                .pattern_coloring_text_use_original
                .clone(),
        })
    }
}

impl LineHandler for LogFormatLineHandler {
    fn handler_type(&self) -> LineHandlerType {
        LineHandlerType::LogFormat
    }

    fn is_active(&self) -> bool {
        if self.pattern_coloring.is_empty() || self.compiled_log_format_regex.as_str().is_empty() {
            return false;
        }

        return true;
    }

    fn process_line(&mut self, line: &mut LineVec) {
        assert!(
            line.len() == 1,
            "LogFormatLineHandler expects a single full line, got {} parts",
            line.len()
        );

        let line_full = &line[0].0;
        let line_original_format = &line[0].1;

        // If nothing matched do nothing.
        let line_matched_groups_res = self.compiled_log_format_regex.captures(line_full);
        if line_matched_groups_res.is_none() {
            return;
        }

        let line_matched_groups = line_matched_groups_res.unwrap();

        // Verify the number of captures match the number of coloring pattern.
        let actual_group_count = line_matched_groups.len() - 1; // 1 for original line
        if actual_group_count != self.pattern_coloring.len() {
            return;
        }

        // Do the actual coloring.
        let mut line_result: LineVec = Vec::new();

        for (i, group) in line_matched_groups.iter().enumerate() {
            // Skip first group which is always a full match.
            if group.is_none() || i == 0 {
                continue;
            }

            let group_str = group.unwrap().as_str();

            let group_bg_color = self.pattern_coloring[i - 1];
            let group_text_color = self.pattern_coloring_text[i - 1];
            let group_text_color_use_original = self.pattern_coloring_text_use_original[i - 1];

            let mut text_format = color_to_text_format_with_textcolor(
                group_bg_color,
                group_text_color,
                self.default_font.clone(),
            );

            if group_text_color_use_original {
                // Preserve original text color.
                text_format.color = line_original_format.color;
            }

            line_result.push((group_str.to_string(), text_format));
        }

        *line = line_result;
    }

    fn points_of_interest(&self) -> Vec<PointOfInterest> {
        Vec::new()
    }
}

pub struct TokenHilightLineHandler {
    token_colors: Vec<(String, Color32)>,
}

impl TokenHilightLineHandler {
    pub fn new(user_settings: &UserSettings) -> Option<Self> {
        if user_settings.token_colors.is_empty() {
            return None;
        }

        let mut token_colors = user_settings.token_colors.clone();

        // Remove all empty or whitespace-only tokens so we don't have to iterate over them later.
        token_colors
            .retain(|(token, _)| !token.is_empty() || !token.chars().all(char::is_whitespace));

        // Sort the token_colors - longest tokens first.
        token_colors.sort_by(|(token_a, _), (token_b, _)| token_b.len().cmp(&token_a.len()));

        Some(Self {
            token_colors: token_colors,
        })
    }
}

impl LineHandler for TokenHilightLineHandler {
    fn handler_type(&self) -> LineHandlerType {
        LineHandlerType::TokenHilight
    }

    fn is_active(&self) -> bool {
        if !self.token_colors.is_empty() {
            return true;
        }

        return false;
    }

    fn process_line(&mut self, line: &mut LineVec) {
        let mut line_result = line.clone();

        for (token, color) in self.token_colors.iter() {
            let split_points = linevec_find(&line_result, token, true, false);
            if split_points.is_empty() {
                continue;
            }

            linevec_split(
                &mut line_result,
                split_points,
                Some(color.clone()),
                Some(calculate_text_color_from_background_color(color.clone())),
            );
        }

        *line = line_result;
    }

    fn points_of_interest(&self) -> Vec<PointOfInterest> {
        Vec::new()
    }
}

pub struct FilterLineHandler {
    filter_term: String,
    match_case: bool,
    whole_word: bool,
    negative: bool,
    extended: bool,
}

impl FilterLineHandler {
    pub fn new(user_settings: &UserSettings) -> Option<Self> {
        if user_settings.filter_term.is_empty() {
            return None;
        }

        Some(Self {
            filter_term: user_settings.filter_term.clone(),
            match_case: user_settings.filter_match_case,
            whole_word: user_settings.filter_whole_word,
            negative: user_settings.filter_negative,
            extended: user_settings.filter_extended,
        })
    }
}

impl LineHandler for FilterLineHandler {
    fn handler_type(&self) -> LineHandlerType {
        LineHandlerType::Filter
    }

    fn is_active(&self) -> bool {
        if self.filter_term.is_empty() {
            return false;
        }

        return true;
    }

    fn process_line(&mut self, line: &mut LineVec) {
        let mut search_terms: Vec<String> = Vec::new();
        let mut is_and_term = false;

        if self.extended {
            // Parse extended filter terms with && and ||.
            // For simplicity, we only support terms with either only "&&"" or only "||" for now.
            if self.filter_term.contains("&&") {
                is_and_term = true;
                for part in self.filter_term.split("&&") {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        search_terms.push(trimmed.to_string());
                    }
                }
            } else if self.filter_term.contains("||") {
                is_and_term = false;
                for part in self.filter_term.split("||") {
                    let trimmed = part.trim();
                    if !trimmed.is_empty() {
                        search_terms.push(trimmed.to_string());
                    }
                }
            } else {
                search_terms.push(self.filter_term.clone());
            }
        } else {
            search_terms.push(self.filter_term.clone());
        }

        let mut matched = if is_and_term { true } else { false };

        for filter_term in search_terms.iter() {
            let split_points = linevec_find(&line, filter_term, self.match_case, self.whole_word);
            let filter_term_matched = !split_points.is_empty();
            if is_and_term {
                matched = matched && filter_term_matched;
                if !matched {
                    // Since we allow only either "AND" or "OR" terms, we can break early here, as
                    // all the rest of the term will evaluate to false anyway.
                    break;
                }
            } else {
                matched = matched || filter_term_matched;
                if matched {
                    // Since we allow only either "AND" or "OR" terms, we can break early here, as
                    // all the rest of the term will evaluate to true anyway.
                    break;
                }
            }
        }

        if !matched {
            // Line does not match, so it should be filtered out.
            if !self.negative {
                // We are not in negative mode, so we clear the line.
                line.clear();
            }
        } else {
            // Line matches, so it should be kept.
            if self.negative {
                // We are in negative mode, so we clear the line.
                line.clear();
            }
        }
    }

    fn points_of_interest(&self) -> Vec<PointOfInterest> {
        Vec::new()
    }
}

pub struct SearchLineHandler {
    search_term: String,
    match_case: bool,
    whole_word: bool,
    points_of_interest: Vec<PointOfInterest>,
}

impl SearchLineHandler {
    pub fn new(user_settings: &UserSettings) -> Option<Self> {
        if user_settings.search_term.is_empty() {
            return None;
        }

        Some(Self {
            search_term: user_settings.search_term.clone(),
            match_case: user_settings.search_match_case,
            whole_word: user_settings.search_whole_word,
            points_of_interest: Vec::new(),
        })
    }
}

impl LineHandler for SearchLineHandler {
    fn handler_type(&self) -> LineHandlerType {
        LineHandlerType::Search
    }

    fn is_active(&self) -> bool {
        if self.search_term.is_empty() {
            return false;
        }

        return true;
    }

    fn process_line(&mut self, line: &mut LineVec) {
        self.points_of_interest.clear(); // Clear previous points of interest.

        let split_points = linevec_find(&line, &self.search_term, self.match_case, self.whole_word);
        if split_points.is_empty() {
            return;
        }

        // Record points of interest.
        for split_point in split_points.iter() {
            let poi = PointOfInterest {
                line: 0,                          // To be filled by caller.
                split_point: split_point.clone(), // This is invalid as soon as the coloring split is done...
            };
            self.points_of_interest.push(poi);
        }

        linevec_split(
            line,
            split_points.clone(),
            Some(Color32::YELLOW),
            Some(Color32::BLACK),
        );
    }

    fn points_of_interest(&self) -> Vec<PointOfInterest> {
        self.points_of_interest.clone()
    }
}
