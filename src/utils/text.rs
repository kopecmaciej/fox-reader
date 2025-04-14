const MIN_SENTENCE_LENGTH: usize = 60;

pub fn split_text_into_sentences(text: &str) -> Vec<String> {
    let mut segments = Vec::new();

    if text.trim().is_empty() {
        return segments;
    }

    let sentence_regex = regex::Regex::new(r"[^.!?]+[.!?]").unwrap();
    for line in text.split('\n') {
        if line.trim().is_empty() {
            continue;
        }

        if line.len() < MIN_SENTENCE_LENGTH {
            segments.push(line.to_string());
            continue;
        }

        let sentence_matches: Vec<_> = sentence_regex.find_iter(line).collect();

        if sentence_matches.is_empty() {
            segments.push(line.to_string());
            continue;
        }

        let line_content = line.to_string();
        segments.push(line_content);
    }

    segments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let text = "";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_single_line_short() {
        let text = "Hello world.";
        let result = split_text_into_sentences(text);
        assert_eq!(result, vec!["Hello world."]);
    }

    #[test]
    fn test_multiple_short_lines() {
        let text = "Line 1.\nLine 2.\nLine 3.";
        let result = split_text_into_sentences(text);
        assert_eq!(result, vec!["Line 1.", "Line 2.", "Line 3."]);
    }

    #[test]
    fn test_single_line_with_multiple_sentences() {
        let text = "This is sentence one. This is sentence two. This is the third one!";
        let result = split_text_into_sentences(text);
        assert_eq!(
            result,
            vec!["This is sentence one. This is sentence two. This is the third one!"]
        );
    }

    #[test]
    fn test_long_line_with_multiple_sentences() {
        let text = "This is a very long first sentence that has a lot of characters, I'm handsome btw. This is a second sentence that also has more than forty characters, I'm handsome here as well";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            "This is a very long first sentence that has a lot of characters, I'm handsome btw. This is a second sentence that also has more than forty characters, I'm handsome here as well"
        );
    }

    #[test]
    fn test_mixed_length_sentences() {
        let text = "Short sentence. This is a much longer sentence with a lot more characters. Another short one.";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0],
            "Short sentence. This is a much longer sentence with a lot more characters. Another short one."
        );
    }

    #[test]
    fn test_with_new_lines() {
        let text = "Short line.\nThis is a longer sentence that should be over 40 characters.\nAnother short line.";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0], "Short line.");
        assert_eq!(
            result[1],
            "This is a longer sentence that should be over 40 characters."
        );
        assert_eq!(result[2], "Another short line.");
    }

    #[test]
    fn test_with_empty_lines() {
        let text = "First line.\n\nThird line.";
        let result = split_text_into_sentences(text);
        assert_eq!(result, vec!["First line.", "Third line."]);
    }

    #[test]
    fn test_with_different_punctuation() {
        let text = "Hello! How are you? I'm fine.";
        let result = split_text_into_sentences(text);
        assert_eq!(result, vec!["Hello! How are you? I'm fine."]);
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
        let text = "Short.\nMedium sentence here.\nThis is a sentence that should be over 40 characters for sure. This is another sentence that's also over 40 characters.\nShort again.\nAnother short.";
        let result = split_text_into_sentences(text);
        assert_eq!(result.len(), 5);
        assert_eq!(result[0], "Short.");
        assert_eq!(result[1], "Medium sentence here.");
        assert_eq!(
            result[2],
            "This is a sentence that should be over 40 characters for sure. This is another sentence that's also over 40 characters."
        );
        assert_eq!(result[3], "Short again.");
        assert_eq!(result[4], "Another short.");
    }
}
