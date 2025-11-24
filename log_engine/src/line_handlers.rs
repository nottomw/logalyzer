use egui::text::TextFormat;
use egui::{Color32, FontId};

use crate::user_settings::UserSettings;

pub trait LineHandler {
    fn is_active(&self) -> bool;
    fn process_line(&self, line: &mut Vec<(String, TextFormat)>);
}

fn color_to_text_format(color_name: egui::Color32, font: FontId) -> TextFormat {
    let mut text_format = TextFormat::default();
    text_format.font_id = font;

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

pub struct LogFormatLineHandler {
    compiled_log_format_regex: regex::Regex,
    pattern_coloring: Vec<Color32>,
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
        })
    }
}

impl LineHandler for LogFormatLineHandler {
    fn is_active(&self) -> bool {
        if self.pattern_coloring.is_empty() || self.compiled_log_format_regex.as_str().is_empty() {
            return false;
        }

        return true;
    }

    fn process_line(&self, line: &mut Vec<(String, TextFormat)>) {
        // Log format works only on full lines.
        assert!(line.len() == 1);

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
            let group_str_coloring = self.pattern_coloring[i - 1];
            let mut text_format =
                color_to_text_format(group_str_coloring, self.default_font.clone());

            text_format.color = line_original_format.color; // Preserve original text color.

            line_result.push((group_str.to_string(), text_format));
        }

        *line = line_result;
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
    fn is_active(&self) -> bool {
        if !self.token_colors.is_empty() {
            return true;
        }

        return false;
    }

    // TODO: token hilighter should be unit tested
    fn process_line(&self, line: &mut Vec<(String, TextFormat)>) {
        let mut line_result: Vec<(String, TextFormat)> = line.clone();

        for (token, color) in self.token_colors.iter() {
            let mut part_no = 0;
            for (part_str, original_text_format) in line_result.iter() {
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
}
