/// Number of lines shown in the gutter: one more than the number of newlines, like VS Code.
pub fn line_count(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count() + 1
}

/// Digits the gutter must fit, never fewer than three so it does not jump while typing.
pub fn gutter_digits(line_count: usize) -> usize {
    line_count.to_string().len().max(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_lines_like_an_editor() {
        assert_eq!(line_count(""), 1);
        assert_eq!(line_count("a"), 1);
        assert_eq!(line_count("a\nb"), 2);
        assert_eq!(line_count("a\n"), 2);
    }

    #[test]
    fn gutter_never_narrower_than_three_digits() {
        assert_eq!(gutter_digits(5), 3);
        assert_eq!(gutter_digits(1000), 4);
    }
}
