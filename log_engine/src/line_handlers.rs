trait LineHandler {
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

struct LogFormatLineHandler {
    compiled_log_format_regex: regex::Regex,
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
            compiled_log_format_regex: regex::Regex::new(&user_settings.log_format.pattern)
                .unwrap(),
        })
    }
}

impl LineHandler for LogFormatLineHandler {
    fn is_active(&self) -> bool {
        if compiled_log_format_regex.is_valid() {
            return true;
        }

        return false;
    }

    fn process_line(&self, line: &mut Vec<(String, TextFormat)>) {
        // Log format works only on full lines.
        assert!(line.len() == 1);

        let line_full = &line[0].0;

        // If nothing matched do nothing.
        let line_matched_groups_res = feature_log_format_regex.captures(line_full);
        if line_matched_groups_res.is_none() {
            return;
        }

        let line_matched_groups = line_matched_groups_res.unwrap();

        // Verify the number of captures match the number of coloring pattern.
        let actual_group_count = line_matched_groups.len() - 1; // 1 for original line
        if actual_group_count != user_settings.log_format.pattern_coloring.len() {
            return;
        }

        // Do the actual coloring.
        let mut line_result: Vec<(String, TextFormat)> = Vec::new();

        for (i, group) in line_matched_groups.iter().enumerate() {
            // Skip first group which is always a full match.
            if group.is_none() || i == 0 {
                return;
            }

            let group_str = group.unwrap().as_str();
            let group_str_coloring = user_settings.log_format.pattern_coloring[i - 1];
            let text_format = color_to_text_format(group_str_coloring, user_settings.font);

            // If this is the last matching group, append a newline.
            if i == line_matched_groups.len() - 1 {
                line_result.push((format!("{}\n", group_str), text_format));
                return;
            }

            line_result.push((group_str.to_string(), text_format));
        }

        *line = line_result;
    }
}

struct TokenHilightLineHandler {
    token_colors: Vec<(String, Color32)>,
    default_format: TextFormat,
}

impl TokenHilightLineHandler {
    pub fn new(user_settings: &UserSettings) -> Option<Self> {
        if user_settings.token_colors.is_empty() {
            return None;
        }

        let mut token_colors = user_settings.token_colors;

        // Remove all empty or whitespace-only tokens so we don't have to iterate over them later.
        token_colors
            .retain(|(token, _)| !token.is_empty() || !token.chars().all(char::is_whitespace));

        Some(Self {
            token_colors: token_colors,
            default_format: TextFormat {
                font_id: user_settings.font,
                ..Default::default()
            },
        })
    }
}

impl LineHandler for LogFormatLineHandler {
    fn is_active(&self) -> bool {
        if !self.token_colors.is_empty() {
            return true;
        }

        return false;
    }

    fn process_line(&self, line: &mut Vec<(String, TextFormat)>) {
        let mut line_result: Vec<(String, TextFormat)> = Vec::new();

        for (part_str, part_format) in line.iter() {
            // Search for a token in each part.
            for (token, color) in self.token_colors.iter() {
                let mut start = 0;

                while let Some(pos) = part_str[start..].find(token) {
                    // Append the text before the token.
                    if pos > 0 {
                        line_result.push((
                            part_str[start..start + pos].to_string(),
                            self.default_format,
                        ));
                    }

                    // Append the token with the highlight color.
                    let highlight_format = color_to_text_format(*color, part_format.font_id);
                    line_result.push((token.to_string(), highlight_format));

                    start += pos + token.len();
                }

                // Append any remaining text after the last token.
                if start < part_str.len() {
                    line_result.push((part_str[start..].to_string(), self.default_format));
                }
            }
        }

        return line_result;
    }
}
