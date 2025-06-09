const MIN_SENTENCE_LENGTH: usize = 10;

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

    // If single line, process for multiple sentences
    if lines.len() == 1 {
        let line = lines[0];
        let sentence_matches: Vec<_> = sentence_regex.find_iter(line).collect();

        if sentence_matches.len() > 1 {
            // Multiple sentences in one line - split them
            for sentence_match in &sentence_matches {
                let sentence = sentence_match.as_str().trim();
                if sentence.len() >= MIN_SENTENCE_LENGTH {
                    segments.push(sentence.to_string());
                }
            }

            // Handle remaining text after last sentence
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
                segments.push(sentence.to_string());
            }
        } else {
            if line.len() >= MIN_SENTENCE_LENGTH {
                if line.len() <= 200 {
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
    } else {
        for line in lines {
            if line.len() >= MIN_SENTENCE_LENGTH {
                segments.push(line.to_string());
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
    fn test_long_line_with_multiple_sentences() {
        let text = "This is a very long first sentence that has a lot of characters, I'm handsome btw. This is a second sentence that also has more than forty characters, I'm handsome here as well.";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            "This is a very long first sentence that has a lot of characters, I'm handsome btw."
        );
        assert_eq!(
            result[1],
            "This is a second sentence that also has more than forty characters, I'm handsome here as well."
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
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], "This is a proper first sentence.");
        assert_eq!(result[1], "This is a medium sentence here.");
        assert_eq!(
            result[2],
            "This is a sentence that should be over 40 characters for sure. This is another sentence that's also over 40 characters."
        );
        assert_eq!(result[3], "This is a proper fourth sentence.");
        assert_eq!(result[4], "This is another proper sentence.");
    }
}
