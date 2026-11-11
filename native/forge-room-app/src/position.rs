/// 1-based (line, column) of a character index, as shown in the status bar.
pub fn line_col(text: &str, char_index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for (index, ch) in text.chars().enumerate() {
        if index >= char_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_lines_and_columns_from_one() {
        assert_eq!(line_col("", 0), (1, 1));
        assert_eq!(line_col("abc", 2), (1, 3));
        assert_eq!(line_col("ab\ncd", 3), (2, 1));
        assert_eq!(line_col("ab\ncd", 5), (2, 3));
    }

    #[test]
    fn counts_characters_not_bytes() {
        assert_eq!(line_col("héllo", 2), (1, 3));
    }
}
