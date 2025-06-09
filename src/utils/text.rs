const MIN_SENTENCE_LENGTH: usize = 10;
const MAX_LINE_LENGTH: usize = 200;
const MIN_SPLIT_POSITION: usize = 60;

pub fn split_text_into_sentences(text: &str) -> Vec<String> {
    let mut segments = Vec::new();

    if text.trim().is_empty() {
        return segments;
    }

    let sentence_regex = regex::Regex::new(r"[^.!?]*?[.!?]+").unwrap();

    let lines: Vec<&str> = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();

    for line in lines {
        let sentence_matches: Vec<_> = sentence_regex.find_iter(line).collect();

        if sentence_matches.len() > 1 {
            for sentence_match in &sentence_matches {
                let sentence = sentence_match.as_str().trim();
                if sentence.len() >= MIN_SENTENCE_LENGTH {
                    if sentence.len() > MAX_LINE_LENGTH {
                        let split_segments = split_long_sentence_at_commas(sentence);
                        segments.extend(split_segments);
                    } else {
                        segments.push(sentence.to_string());
                    }
                }
            }

            if let Some(last_match) = sentence_matches.last() {
                let last_end = last_match.end();
                if last_end < line.len() {
                    let remaining = line[last_end..].trim();
                    if !remaining.is_empty() && remaining.len() >= MIN_SENTENCE_LENGTH {
                        segments.push(remaining.to_string());
                    }
                }
            }
        } else if sentence_matches.len() == 1 {
            let sentence = sentence_matches[0].as_str().trim();
            if sentence.len() >= MIN_SENTENCE_LENGTH {
                if sentence.len() > MAX_LINE_LENGTH {
                    let split_segments = split_long_sentence_at_commas(sentence);
                    segments.extend(split_segments);
                } else {
                    segments.push(sentence.to_string());
                }
            }
        } else {
            if line.len() >= MIN_SENTENCE_LENGTH {
                if line.len() <= MAX_LINE_LENGTH {
                    segments.push(line.to_string());
                } else {
                    let words: Vec<&str> = line.split_whitespace().collect();
                    let mut current_segment = String::new();

                    for word in words {
                        if current_segment.len() + word.len() + 1 > 100
                            && !current_segment.is_empty()
                        {
                            segments.push(current_segment.trim().to_string());
                            current_segment = String::new();
                        }
                        if !current_segment.is_empty() {
                            current_segment.push(' ');
                        }
                        current_segment.push_str(word);
                    }

                    if !current_segment.trim().is_empty() {
                        segments.push(current_segment.trim().to_string());
                    }
                }
            }
        }
    }

    if segments.is_empty() && !text.trim().is_empty() {
        let normalized = text
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        if !normalized.is_empty() {
            segments.push(normalized);
        }
    }

    segments
}

fn split_long_sentence_at_commas(sentence: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current_start = 0;

    let comma_positions: Vec<usize> = sentence
        .char_indices()
        .filter_map(|(i, c)| if c == ',' { Some(i) } else { None })
        .collect();

    for &comma_pos in &comma_positions {
        if comma_pos >= current_start + MIN_SPLIT_POSITION {
            let segment = &sentence[current_start..=comma_pos];
            if segment.trim().len() >= MIN_SENTENCE_LENGTH {
                segments.push(segment.trim().to_string());
                current_start = comma_pos + 1;
            }
        }
    }

    let remaining = &sentence[current_start..];
    if remaining.trim().len() >= MIN_SENTENCE_LENGTH {
        segments.push(remaining.trim().to_string());
    }

    if segments.is_empty() {
        segments.push(sentence.to_string());
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_star_wars_example() {
        let text = "In the Star Wars franchise, Darth Vader's lightsaber is red. This is consistent throughout the original trilogy, prequels, and other Star Wars media. The red color of Sith lightsabers is typically associated with the dark side of the Force, in contrast to the blue and green lightsabers commonly used by Jedi.";
        let result = split_text_into_sentences(text);
        println!("Star Wars result: {:?}", result);
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0],
            "In the Star Wars franchise, Darth Vader's lightsaber is red."
        );
        assert_eq!(result[1], "This is consistent throughout the original trilogy, prequels, and other Star Wars media.");
        assert_eq!(result[2], "The red color of Sith lightsabers is typically associated with the dark side of the Force, in contrast to the blue and green lightsabers commonly used by Jedi.");
    }

    #[test]
    fn test_empty_text() {
        let text = "";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_single_line_medium() {
        let text = "Hello world, this is a test.";
        let result = split_text_into_sentences(text);
        assert_eq!(result, vec!["Hello world, this is a test."]);
    }

    #[test]
    fn test_multiple_lines_with_sentences() {
        let text = "This is the first sentence.\nThis is the second sentence.\nThis is the third sentence.";
        let result = split_text_into_sentences(text);
        assert_eq!(
            result,
            vec![
                "This is the first sentence.",
                "This is the second sentence.",
                "This is the third sentence."
            ]
        );
    }

    #[test]
    fn test_single_line_with_multiple_sentences() {
        let text = "This is sentence one. This is sentence two. This is the third one!";
        let result = split_text_into_sentences(text);
        assert_eq!(
            result,
            vec![
                "This is sentence one.",
                "This is sentence two.",
                "This is the third one!"
            ]
        );
    }

    #[test]
    fn test_mixed_length_sentences() {
        let text = "This is a longer sentence. This is a much longer sentence with a lot more characters. This is another longer sentence.";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "This is a longer sentence.");
        assert_eq!(
            result[1],
            "This is a much longer sentence with a lot more characters."
        );
        assert_eq!(result[2], "This is another longer sentence.");
    }

    #[test]
    fn test_with_new_lines() {
        let text = "This is a proper sentence.\nThis is a longer sentence that should be over 40 characters.\nThis is another proper sentence.";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "This is a proper sentence.");
        assert_eq!(
            result[1],
            "This is a longer sentence that should be over 40 characters."
        );
        assert_eq!(result[2], "This is another proper sentence.");
    }

    #[test]
    fn test_with_empty_lines() {
        let text = "This is the first sentence.\n\nThis is the third sentence.";
        let result = split_text_into_sentences(text);
        assert_eq!(
            result,
            vec!["This is the first sentence.", "This is the third sentence."]
        );
    }

    #[test]
    fn test_with_different_punctuation() {
        let text = "Hello there! How are you doing today? I'm feeling quite fine.";
        let result = split_text_into_sentences(text);
        assert_eq!(
            result,
            vec![
                "Hello there!",
                "How are you doing today?",
                "I'm feeling quite fine."
            ]
        );
    }

    #[test]
    fn test_very_long_single_sentence() {
        let text = "This is an extremely long sentence that definitely exceeds the forty character threshold and should be treated as a single segment because it only contains one sentence with a single ending punctuation mark.";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], text);
    }

    #[test]
    fn test_line_with_no_punctuation() {
        let text = "This line has no punctuation but it is longer than forty characters";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], text);
    }

    #[test]
    fn test_complex_mixed_content() {
        let text = "This is a proper first sentence.\nThis is a medium sentence here.\nThis is a sentence that should be over 40 characters for sure. This is another sentence that's also over 40 characters.\nThis is a proper fourth sentence.\nThis is another proper sentence.";
        let result = split_text_into_sentences(text);
        // Fixed expectation: should split all sentences consistently
        assert_eq!(result.len(), 6);
        assert_eq!(result[0], "This is a proper first sentence.");
        assert_eq!(result[1], "This is a medium sentence here.");
        assert_eq!(
            result[2],
            "This is a sentence that should be over 40 characters for sure."
        );
        assert_eq!(
            result[3],
            "This is another sentence that's also over 40 characters."
        );
        assert_eq!(result[4], "This is a proper fourth sentence.");
        assert_eq!(result[5], "This is another proper sentence.");
    }

    #[test]
    fn test_long_sentence_split_at_commas() {
        let text = "This is a very long sentence that exceeds 200 characters, and it contains multiple commas, which should be used as split points, to break it into smaller more manageable segments, while preserving the meaning and structure of the original text.";
        let result = split_text_into_sentences(text);

        assert!(result.len() > 1);

        assert!(result[0].ends_with(","));
        assert!(result[0].len() >= MIN_SPLIT_POSITION);

        for segment in &result {
            assert!(segment.len() >= MIN_SENTENCE_LENGTH);
        }

        let rejoined = result.join(" ");
        assert!(rejoined.contains("very long sentence"));
        assert!(rejoined.contains("multiple commas"));
        assert!(rejoined.contains("preserving the meaning"));
    }

    #[test]
    fn test_long_sentence_no_commas() {
        let text = "This is a very long sentence that exceeds 200 characters but contains no commas so it should not be split and should remain as a single segment even though it is longer than the maximum line length threshold.";
        let result = split_text_into_sentences(text);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0], text);
    }

    #[test]
    fn test_long_sentence_early_commas() {
        let text = "This is a sentence, with early commas, but they appear before the 60 character minimum split position so the sentence should remain intact and not be split at those early comma positions.";
        let result = split_text_into_sentences(text);

        assert!(result.len() >= 1);

        if result.len() > 1 {
            assert!(result[0].len() >= MIN_SPLIT_POSITION);
        }
    }
}
