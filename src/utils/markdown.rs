use regex::Regex;

pub fn strip_markdown_for_tts(text: &str) -> String {
    let mut result = text.to_string();

    let code_block_regex = Regex::new(r"```[\s\S]*?```").unwrap();
    result = code_block_regex
        .replace_all(&result, "Code snippet available in chat.")
        .to_string();

    let inline_code_regex = Regex::new(r"`([^`]+)`").unwrap();
    result = inline_code_regex.replace_all(&result, "$1").to_string();

    let header_regex = Regex::new(r"#{1,6}\s+(.+)").unwrap();
    result = header_regex.replace_all(&result, "$1").to_string();

    let asterisk_regex = Regex::new(r"\*+").unwrap();
    result = asterisk_regex.replace_all(&result, "").to_string();

    let link_regex = Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap();
    result = link_regex.replace_all(&result, "$1").to_string();

    let bullet_regex = Regex::new(r"^\s*[-*+]\s+").unwrap();
    let numbered_regex = Regex::new(r"^\s*\d+\.\s+").unwrap();
    let blockquote_regex = Regex::new(r"^\s*>\s+(.+)").unwrap();

    result = result
        .lines()
        .map(|line| {
            let line = bullet_regex.replace(line, "").to_string();
            let line = numbered_regex.replace(&line, "").to_string();
            let line = blockquote_regex.replace(&line, "$1").to_string();
            line
        })
        .collect::<Vec<String>>()
        .join("\n");

    let hr_regex = Regex::new(r"(?m)^\s*[-*_]{3,}\s*$").unwrap();
    result = hr_regex.replace_all(&result, "").to_string();

    let underscore_regex = Regex::new(r"_+").unwrap();
    result = underscore_regex.replace_all(&result, "").to_string();

    let strikethrough_regex = Regex::new(r"~~(.+?)~~").unwrap();
    result = strikethrough_regex.replace_all(&result, "$1").to_string();

    let newline_regex = Regex::new(r"\n{3,}").unwrap();
    result = newline_regex.replace_all(&result, "\n\n").to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markdown() {
        let markdown = r#"# Header
        
This is **bold** and *italic* text.

```rust
fn main() {
    println!("Hello, world!");
}
```

- List item 1
- List item 2

[Link text](https://example.com)

> Blockquote text

Inline `code` here.

---

Normal text."#;

        let expected = r#"Header
        
This is bold and italic text.

Code snippet available in chat.

List item 1
List item 2

Link text

Blockquote text

Inline code here.

Normal text."#;

        assert_eq!(strip_markdown_for_tts(markdown), expected);
    }

    #[test]
    fn test_strip_multiple_code_blocks() {
        let markdown = r#"Here is some code:
        
```python
def hello():
    print("Hello world")
```

And more code:

```javascript
console.log("Hello world");
```"#;

        let expected = r#"Here is some code:
        
Code snippet available in chat.

And more code:

Code snippet available in chat."#;

        assert_eq!(strip_markdown_for_tts(markdown), expected);
    }

    #[test]
    fn test_nested_formatting() {
        let markdown1 = r#"This is **bold with *italic* inside**"#;
        let expected1 = r#"This is bold with italic inside"#;
        assert_eq!(strip_markdown_for_tts(markdown1), expected1);

        let markdown2 = r#"This is *italic with **bold** inside*"#;
        let expected2 = r#"This is italic with bold inside"#;
        assert_eq!(strip_markdown_for_tts(markdown2), expected2);

        let markdown3 =
            r#"This is **bold with *italic* inside** and *italic with **bold** inside*"#;
        let expected3 = r#"This is bold with italic inside and italic with bold inside"#;
        assert_eq!(strip_markdown_for_tts(markdown3), expected3);
    }

    #[test]
    fn test_horizontal_rules() {
        let markdown = r#"Text before

---

****

___

Text after"#;

        let expected = r#"Text before

Text after"#;

        assert_eq!(strip_markdown_for_tts(markdown), expected);
    }

    #[test]
    fn test_nested_lists() {
        let markdown = r#"- Main item 1
  - Sub item 1.1
  - Sub item 1.2
- Main item 2
  1. Numbered sub 2.1
  2. Numbered sub 2.2"#;

        let expected = r#"Main item 1
Sub item 1.1
Sub item 1.2
Main item 2
Numbered sub 2.1
Numbered sub 2.2"#;

        assert_eq!(strip_markdown_for_tts(markdown), expected);
    }

    #[test]
    fn test_underline_as_italic() {
        let markdown = r#"This is _underlined_ text which is treated as italic in markdown"#;
        let expected = r#"This is underlined text which is treated as italic in markdown"#;

        assert_eq!(strip_markdown_for_tts(markdown), expected);
    }

    #[test]
    fn test_strikethrough() {
        let markdown = r#"This is ~~strikethrough~~ text"#;
        let expected = r#"This is strikethrough text"#;

        assert_eq!(strip_markdown_for_tts(markdown), expected);
    }
}
