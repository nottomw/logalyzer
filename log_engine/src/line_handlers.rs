use egui::text::TextFormat;
use egui::{Color32, FontId};

use crate::PointOfInterest;
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
    fn process_line(&mut self, line: &mut Vec<(String, TextFormat)>);
    fn points_of_interest(&self) -> Vec<PointOfInterest>;
}

fn color_to_text_format(color_background: egui::Color32, font: FontId) -> TextFormat {
    let mut text_format = TextFormat::default();
    text_format.font_id = font;

    text_format.background = color_background;

    // Ensure the text color is visible on the background.
    // If it's bright, make the color black, else white.
    text_format.color = if (color_background.r() as u32
        + color_background.g() as u32
        + color_background.b() as u32)
        / 3
        > 128
    {
        Color32::BLACK
    } else {
        Color32::WHITE
    };

    text_format
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

// TODO: build some primitives for LineHandlers, like splitting lines into parts,
//       hilighting some parts, searching for text in split line, ...
// type LineVec = Vec<(String, TextFormat)>;
// type SplitPoint = (usize, usize, usize); // (index in linevec, offset in part, splitting length)

// fn lh_find(line: &LineVec, search_term: &str) -> Vec<SplitPoint> {
//     Vec::new()
// }

// fn lh_split(line: &mut LineVec, split_point: SplitPoint) {}

// fn lh_split_and_color(
//     line: &mut LineVec,
//     split_point: SplitPoint,
//     color_bg: egui::Color32,
//     color_text: egui::Color32,
// ) {
// }

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

    fn process_line(&mut self, line: &mut Vec<(String, TextFormat)>) {
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
        let mut line_result: Vec<(String, TextFormat)> = Vec::new();

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

    // TODO: token hilighter should be unit tested
    fn process_line(&mut self, line: &mut Vec<(String, TextFormat)>) {
        let mut line_result: Vec<(String, TextFormat)> = line.clone();

        for (token, color) in self.token_colors.iter() {
            let mut part_no = 0;
            for (part_str, original_text_format) in line_result.iter() {
                // TODO: if the search term spans multiple parts, it will not be found. Should be fixed.
                if part_str.contains(token) {
                    let mut new_line_result: Vec<(String, TextFormat)> = line_result.clone();
                    let mut start = 0;

                    let mut part_no_offset = 0;

                    while let Some(pos) = part_str[start..].find(token) {
                        // Append the text before the token.
                        if pos > 0 {
                            if part_no_offset == 0 {
                                new_line_result[part_no + part_no_offset] = (
                                    part_str[start..start + pos].to_string(),
                                    original_text_format.clone(),
                                );
                            } else {
                                new_line_result.insert(
                                    part_no + part_no_offset,
                                    (
                                        part_str[start..start + pos].to_string(),
                                        original_text_format.clone(),
                                    ),
                                );
                            }
                            part_no_offset += 1;
                        }

                        // Append the token with the highlight color.
                        let highlight_format =
                            color_to_text_format(*color, original_text_format.font_id.clone());
                        new_line_result.insert(
                            part_no + part_no_offset,
                            (token.to_string(), highlight_format),
                        );
                        part_no_offset += 1;

                        start += pos + token.len();
                    }

                    // Append any remaining text after the last token.
                    if start < part_str.len() {
                        new_line_result.insert(
                            part_no + part_no_offset,
                            (part_str[start..].to_string(), original_text_format.clone()),
                        );
                    }

                    line_result = new_line_result;
                    break; // Move to the next token after processing this one.
                }

                part_no += 1;
            }
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

    fn process_line(&mut self, line: &mut Vec<(String, TextFormat)>) {
        let mut matched = false;
        // TODO: if the search term spans multiple parts, it will not be found. Should be fixed.
        for (part_str, _) in line.iter() {
            let haystack = if self.match_case {
                part_str.to_string()
            } else {
                part_str.to_lowercase()
            };

            let needle = if self.match_case {
                self.filter_term.clone()
            } else {
                self.filter_term.to_lowercase()
            };

            if self.whole_word {
                let words: Vec<&str> = haystack
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .collect();
                if words.iter().any(|&word| word == needle) {
                    matched = true;
                    break;
                }
            } else {
                if haystack.contains(&needle) {
                    matched = true;
                    break;
                }
            }
        }

        if !matched {
            if self.negative {
                return;
            } else {
                line.clear();
            }
        } else {
            if self.negative {
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

    fn process_line(&mut self, line: &mut Vec<(String, TextFormat)>) {
        self.points_of_interest.clear(); // Clear previous points of interest.

        for i in 0..line.len() {
            // TODO: if the search term spans multiple parts, it will not be found. Should be fixed.
            let part_str = &line[i].0;
            let haystack = if self.match_case {
                part_str.to_string()
            } else {
                part_str.to_lowercase()
            };

            let needle = if self.match_case {
                self.search_term.clone()
            } else {
                self.search_term.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = haystack[start..].find(&needle) {
                // If whole_word is enabled, verify the match is a whole word.
                if self.whole_word {
                    let before_char = if pos + start == 0 {
                        ' ' // Treat start of string as non-alphanumeric
                    } else {
                        haystack.chars().nth(pos + start - 1).unwrap()
                    };
                    let after_char = if pos + start + needle.len() >= haystack.len() {
                        ' ' // Treat end of string as non-alphanumeric
                    } else {
                        haystack.chars().nth(pos + start + needle.len()).unwrap()
                    };

                    if before_char.is_alphanumeric() || after_char.is_alphanumeric() {
                        start += pos + 1;
                        continue; // Not a whole word match
                    }
                }

                self.points_of_interest.push(PointOfInterest {
                    line: 0, // This will be set by the caller,
                    line_part_index: i,
                    line_offset: pos + start,
                    line_point_size: needle.len(),
                });

                start += pos + needle.len();
            }

            // Hilight the search terms.
            if !self.points_of_interest.is_empty() {
                let mut new_line_parts: Vec<(String, TextFormat)> = Vec::new();
                let original_text_format = &line[i].1;

                let mut last_index = 0;
                for poi in self
                    .points_of_interest
                    .iter()
                    .filter(|p| p.line_part_index == i)
                {
                    // Append text before the match.
                    if poi.line_offset > last_index {
                        new_line_parts.push((
                            part_str[last_index..poi.line_offset].to_string(),
                            original_text_format.clone(),
                        ));
                    }

                    // Append the matched term with highlight.
                    let mut highlight_format = original_text_format.clone();
                    highlight_format.background = Color32::YELLOW; // Highlight color
                    new_line_parts.push((
                        part_str[poi.line_offset..poi.line_offset + poi.line_point_size]
                            .to_string(),
                        highlight_format,
                    ));

                    last_index = poi.line_offset + poi.line_point_size;
                }

                // Append any remaining text after the last match.
                if last_index < part_str.len() {
                    new_line_parts.push((
                        part_str[last_index..].to_string(),
                        original_text_format.clone(),
                    ));
                }

                // Replace the original part with the new parts.
                line.remove(i);
                for (j, new_part) in new_line_parts.into_iter().enumerate() {
                    line.insert(i + j, new_part);
                }
            }
        }
    }

    fn points_of_interest(&self) -> Vec<PointOfInterest> {
        self.points_of_interest.clone()
    }
}
