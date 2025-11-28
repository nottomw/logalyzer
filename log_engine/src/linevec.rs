use egui::TextFormat;

pub type LineVec = Vec<(String, TextFormat)>;
pub type SplitPointPartial = (usize, usize); // (index in linevec, starting/ending offset in part)
pub type SplitPoint = (SplitPointPartial, SplitPointPartial); // (start split, end split)

pub fn linevec_find(
    line: &LineVec,
    search_term: &str,
    match_case: bool,
    match_whole_word: bool,
) -> Vec<SplitPoint> {
    let combined_str = if match_case {
        line.iter().map(|(s, _)| s.as_str()).collect::<String>()
    } else {
        line.iter()
            .map(|(s, _)| s.to_lowercase())
            .collect::<String>()
    };

    let search_term_adjusted = if match_case {
        search_term.to_string()
    } else {
        search_term.to_lowercase()
    };

    let mut parts_offsets = Vec::new();
    let mut current_offset = 0;
    for (i, (part_str, _)) in line.iter().enumerate() {
        let part_len = part_str.len();
        parts_offsets.push((i, current_offset, current_offset + part_len));
        current_offset += part_len;
    }

    let mut split_points = Vec::new();
    let mut search_start = 0;

    while let Some(pos) = combined_str[search_start..].find(&search_term_adjusted) {
        let actual_pos = search_start + pos;

        if match_whole_word {
            let is_start_boundary = actual_pos == 0
                || !combined_str
                    .chars()
                    .nth(actual_pos - 1)
                    .unwrap()
                    .is_alphanumeric();
            let is_end_boundary = actual_pos + search_term.len() == combined_str.len()
                || !combined_str
                    .chars()
                    .nth(actual_pos + search_term.len())
                    .unwrap()
                    .is_alphanumeric();

            if !is_start_boundary || !is_end_boundary {
                search_start = actual_pos + 1;
                continue;
            }
        }

        let mut start_split: SplitPointPartial = (0, 0);
        let mut end_split: SplitPointPartial = (0, 0);

        for (i, part_start, part_end) in &parts_offsets {
            if actual_pos >= *part_start && actual_pos < *part_end {
                start_split = (*i, actual_pos - part_start);
            }

            if actual_pos + search_term.len() > *part_start
                && actual_pos + search_term.len() <= *part_end
            {
                end_split = (*i, actual_pos + search_term.len() - part_start);
            }
        }

        split_points.push((start_split, end_split));
        search_start = actual_pos + search_term.len();
    }

    split_points
}

pub fn linevec_split(
    line: &mut LineVec,
    split_points: Vec<SplitPoint>,
    middle_color_bg: Option<egui::Color32>,
    middle_color_text: Option<egui::Color32>,
) {
    let mut split_points = split_points;
    split_points.sort_by_key(|(start, _)| *start);

    let middle_text_format = |original_format: &TextFormat| {
        let mut new_format = original_format.clone();
        if let Some(bg) = middle_color_bg {
            new_format.background = bg;
        }

        if let Some(text) = middle_color_text {
            new_format.color = text;
        }

        new_format
    };

    for split_point in split_points.into_iter().rev() {
        let splitpoint_start = split_point.0;
        let splitpoint_end = split_point.1;

        let splitpoint_start_index = splitpoint_start.0;
        let splitpoint_start_offset = splitpoint_start.1;
        let splitpoint_end_index = splitpoint_end.0;
        let splitpoint_end_offset = splitpoint_end.1;

        assert!(splitpoint_start_index <= splitpoint_end_index);
        assert!(splitpoint_start_index < line.len());
        assert!(splitpoint_end_index < line.len());

        let part = &mut line[splitpoint_start_index];
        if splitpoint_start_index == splitpoint_end_index {
            let original_text = part.0.clone();
            let original_format = part.1.clone();

            let before_text = original_text[..splitpoint_start_offset].to_string();
            let middle_text =
                original_text[splitpoint_start_offset..splitpoint_end_offset].to_string();
            let after_text = original_text[splitpoint_end_offset..].to_string();

            *part = (before_text, original_format.clone());

            line.insert(
                splitpoint_start_index + 1,
                (middle_text, middle_text_format(&original_format)),
            );

            line.insert(splitpoint_start_index + 2, (after_text, original_format));
        } else {
            // Split start part
            let original_text_start = part.0.clone();
            let original_format_start = part.1.clone();

            let before_text = original_text_start[..splitpoint_start_offset].to_string();
            let middle_text_start = original_text_start[splitpoint_start_offset..].to_string();

            *part = (before_text, original_format_start.clone());

            line.insert(
                splitpoint_start_index + 1,
                (
                    middle_text_start,
                    middle_text_format(&original_format_start),
                ),
            );

            // If there are middle parts and they need to be colored, do it now.
            for middle_part_index in (splitpoint_start_index + 1)..splitpoint_end_index {
                let middle_part = &mut line[middle_part_index];
                let original_text_middle = middle_part.0.clone();
                let original_format_middle = middle_part.1.clone();

                *middle_part = (
                    original_text_middle,
                    middle_text_format(&original_format_middle),
                );
            }

            // Split end part
            let part_end = &mut line[splitpoint_end_index + 1];
            let original_text_end = part_end.0.clone();
            let original_format_end = part_end.1.clone();

            let middle_text_end = original_text_end[..splitpoint_end_offset].to_string();
            let after_text = original_text_end[splitpoint_end_offset..].to_string();

            *part_end = (middle_text_end, middle_text_format(&original_format_end));

            line.insert(splitpoint_end_index + 2, (after_text, original_format_end));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    #[test]
    fn basic_string_searches() {
        let mut line: LineVec = vec![("Hello world".to_string(), TextFormat::default())];

        let mut split_points = linevec_find(&line, "lo wo", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 3), (0, 8)));

        split_points = linevec_find(&line, "Hello", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 0), (0, 5)));

        split_points = linevec_find(&line, "world", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 6), (0, 11)));
    }

    #[test]
    fn multi_part_string_searches_simple() {
        let mut line: LineVec = vec![
            ("Hello ".to_string(), TextFormat::default()),
            ("cruel".to_string(), TextFormat::default()),
            ("world".to_string(), TextFormat::default()),
        ];

        let mut split_points = linevec_find(&line, "Hello", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 0), (0, 5)));

        split_points = linevec_find(&line, "cruel", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((1, 0), (1, 5)));

        split_points = linevec_find(&line, "world", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((2, 0), (2, 5)));
    }

    #[test]
    fn multi_part_string_searches_across_parts() {
        let mut line: LineVec = vec![
            ("Hello ".to_string(), TextFormat::default()),
            ("cruel ".to_string(), TextFormat::default()),
            ("world".to_string(), TextFormat::default()),
        ];

        let mut split_points = linevec_find(&line, "lo cru", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 3), (1, 3)));

        split_points = linevec_find(&line, "cruel wo", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((1, 0), (2, 2)));

        split_points = linevec_find(&line, "Hello cruel world", true, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 0), (2, 5)));
    }

    #[test]
    fn multi_part_string_searches_multiple_occurrences() {
        let mut line: LineVec = vec![
            ("Hello ".to_string(), TextFormat::default()),
            ("cruel ".to_string(), TextFormat::default()),
            ("world".to_string(), TextFormat::default()),
            ("Hello ".to_string(), TextFormat::default()),
            ("world".to_string(), TextFormat::default()),
        ];

        let mut split_points = linevec_find(&line, "lo", true, false);
        assert_eq!(split_points.len(), 2);
        assert_eq!(split_points[0], ((0, 3), (0, 5)));
        assert_eq!(split_points[1], ((3, 3), (3, 5)));

        split_points = linevec_find(&line, "world", true, false);
        assert_eq!(split_points.len(), 2);
        assert_eq!(split_points[0], ((2, 0), (2, 5)));
        assert_eq!(split_points[1], ((4, 0), (4, 5)));

        split_points = linevec_find(&line, "o", true, false);
        assert_eq!(split_points.len(), 4);
        assert_eq!(split_points[0], ((0, 4), (0, 5)));
        assert_eq!(split_points[1], ((2, 1), (2, 2)));
        assert_eq!(split_points[2], ((3, 4), (3, 5)));
        assert_eq!(split_points[3], ((4, 1), (4, 2)));
    }

    #[test]
    fn multi_part_string_searches_across_parts_multiple_occurrences() {
        let mut line: LineVec = vec![
            ("ab".to_string(), TextFormat::default()),
            ("cd".to_string(), TextFormat::default()),
            ("ab".to_string(), TextFormat::default()),
            ("cd".to_string(), TextFormat::default()),
        ];

        let mut split_points = linevec_find(&line, "bc", true, false);
        assert_eq!(split_points.len(), 2);
        assert_eq!(split_points[0], ((0, 1), (1, 1)));
        assert_eq!(split_points[1], ((2, 1), (3, 1)));
    }

    #[test]
    fn case_insensitive_searches() {
        let mut line: LineVec = vec![("Hello World".to_string(), TextFormat::default())];

        let mut split_points = linevec_find(&line, "hello", false, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 0), (0, 5)));

        split_points = linevec_find(&line, "WORLD", false, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 6), (0, 11)));

        split_points = linevec_find(&line, "Lo Wo", false, false);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 3), (0, 8)));
    }

    #[test]
    fn whole_word_searches() {
        let mut line: LineVec = vec![(
            "Hello world, hello universe".to_string(),
            TextFormat::default(),
        )];

        let mut split_points = linevec_find(&line, "hello", true, true);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 13), (0, 18)));

        split_points = linevec_find(&line, "world", true, true);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 6), (0, 11)));

        split_points = linevec_find(&line, "lo", true, true);
        assert_eq!(split_points.len(), 0);
    }

    #[test]
    fn whole_word_searches_across_parts() {
        let mut line: LineVec = vec![
            ("lorem ip".to_string(), TextFormat::default()),
            ("sum, consecteur ".to_string(), TextFormat::default()),
            ("adipiscit el".to_string(), TextFormat::default()),
            ("it".to_string(), TextFormat::default()),
        ];

        let mut split_points = linevec_find(&line, "ipsum", true, true);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((0, 6), (1, 3)));

        split_points = linevec_find(&line, "elit", true, true);
        assert_eq!(split_points.len(), 1);
        assert_eq!(split_points[0], ((2, 10), (3, 2)));

        split_points = linevec_find(&line, "teur adipiscit", true, true);
        assert_eq!(split_points.len(), 0);
    }

    #[test]
    fn basic_split() {
        let mut line: LineVec = vec![("Hello world".to_string(), TextFormat::default())];

        let split_points = vec![((0, 3), (0, 8))];
        linevec_split(&mut line, split_points, None, None);

        assert_eq!(line.len(), 3);
        assert_eq!(line[0], ("Hel".to_string(), TextFormat::default()));
        assert_eq!(line[1], ("lo wo".to_string(), TextFormat::default()));
        assert_eq!(line[2], ("rld".to_string(), TextFormat::default()));
    }

    #[test]
    fn multi_part_split() {
        let mut line: LineVec = vec![
            ("Hello ".to_string(), TextFormat::default()),
            ("cruel ".to_string(), TextFormat::default()),
            ("world".to_string(), TextFormat::default()),
        ];

        let split_points = vec![((0, 3), (1, 3))];
        linevec_split(&mut line, split_points, None, None);

        assert_eq!(line.len(), 5);
        assert_eq!(line[0], ("Hel".to_string(), TextFormat::default()));
        assert_eq!(line[1], ("lo ".to_string(), TextFormat::default()));
        assert_eq!(line[2], ("cru".to_string(), TextFormat::default()));
        assert_eq!(line[3], ("el ".to_string(), TextFormat::default()));
        assert_eq!(line[4], ("world".to_string(), TextFormat::default()));
    }

    #[test]
    fn basic_split_with_coloring() {
        let mut line: LineVec = vec![("Hello world".to_string(), TextFormat::default())];

        let split_points = vec![((0, 3), (0, 8))];
        linevec_split(
            &mut line,
            split_points,
            Some(Color32::RED),
            Some(Color32::WHITE),
        );

        assert_eq!(line.len(), 3);
        assert_eq!(line[0], ("Hel".to_string(), TextFormat::default()));
        assert_eq!(
            line[1],
            (
                "lo wo".to_string(),
                TextFormat {
                    background: Color32::RED,
                    color: Color32::WHITE,
                    ..Default::default()
                }
            )
        );
        assert_eq!(line[2], ("rld".to_string(), TextFormat::default()));
    }

    #[test]
    fn multi_part_split_with_coloring() {
        let mut line: LineVec = vec![
            ("Hello ".to_string(), TextFormat::default()),
            ("cruel ".to_string(), TextFormat::default()),
            ("world".to_string(), TextFormat::default()),
        ];

        let split_points = vec![((0, 3), (1, 3))];
        linevec_split(
            &mut line,
            split_points,
            Some(Color32::RED),
            Some(Color32::WHITE),
        );

        assert_eq!(line.len(), 5);
        assert_eq!(line[0], ("Hel".to_string(), TextFormat::default()));
        assert_eq!(
            line[1],
            (
                "lo ".to_string(),
                TextFormat {
                    background: Color32::RED,
                    color: Color32::WHITE,
                    ..Default::default()
                }
            )
        );
        assert_eq!(
            line[2],
            (
                "cru".to_string(),
                TextFormat {
                    background: Color32::RED,
                    color: Color32::WHITE,
                    ..Default::default()
                }
            )
        );
        assert_eq!(line[3], ("el ".to_string(), TextFormat::default()));
        assert_eq!(line[4], ("world".to_string(), TextFormat::default()));
    }
}
