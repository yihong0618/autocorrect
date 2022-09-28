use serde::{Deserialize, Serialize};
use serde_repr::*;

#[derive(Serialize_repr, Deserialize_repr, PartialEq, Eq)]
#[repr(u8)]
pub enum Severity {
    Pass = 0,
    Error = 1,
    Warning = 2,
}

impl Severity {
    pub fn is_error(&self) -> bool {
        self == &Severity::Error
    }

    pub fn is_warning(&self) -> bool {
        self == &Severity::Warning
    }
}

#[derive(Serialize, Deserialize)]
pub struct LineResult {
    #[serde(rename(serialize = "l"))]
    pub line: usize,
    #[serde(rename(serialize = "c"))]
    pub col: usize,
    pub new: String,
    pub old: String,
    pub severity: Severity,
}

pub trait Results {
    fn push(&mut self, line_result: LineResult);
    fn ignore(&mut self, str: &str);
    fn error(&mut self, err: &str);
    fn to_string(&self) -> String;
    fn is_lint(&self) -> bool;
    /// Toggle AutoCorrrect template enable or disable
    fn toggle(&mut self, enable: bool);
    /// Is AutoCorrrect current is enable
    fn is_enabled(&self) -> bool;
    /// Move and save current line,col return the previus line number
    fn move_cursor(&mut self, part: &str) -> (usize, usize);
}

#[derive(Serialize, Deserialize)]
pub struct FormatResult {
    pub out: String,
    pub error: String,
    #[serde(skip)]
    pub raw: String,
    #[serde(skip)]
    pub enable: bool,
}

#[derive(Serialize, Deserialize)]
pub struct LintResult {
    #[serde(skip)]
    pub raw: String,
    pub filepath: String,
    pub lines: Vec<LineResult>,
    pub error: String,
    #[serde(skip)]
    pub enable: bool,
    // For store line number in loop
    #[serde(skip)]
    line: usize,
    // For store col number in loop
    #[serde(skip)]
    col: usize,
}

impl FormatResult {
    pub fn new(raw: &str) -> Self {
        FormatResult {
            raw: String::from(raw),
            out: String::from(""),
            error: String::from(""),
            enable: true,
        }
    }

    #[allow(dead_code)]
    pub fn has_error(&self) -> bool {
        !self.error.is_empty()
    }
}

impl Results for FormatResult {
    fn push(&mut self, line_result: LineResult) {
        self.out.push_str(line_result.new.as_str());
    }

    fn ignore(&mut self, part: &str) {
        self.out.push_str(part);
        self.move_cursor(part);
    }

    fn error(&mut self, err: &str) {
        // Revert out to raw when has error, make sure return raw value.
        self.out = self.raw.clone();
        self.error = String::from(err);
    }

    fn to_string(&self) -> String {
        self.out.to_string()
    }

    fn is_lint(&self) -> bool {
        false
    }

    fn toggle(&mut self, enable: bool) {
        self.enable = enable;
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    fn move_cursor(&mut self, _part: &str) -> (usize, usize) {
        (0, 0)
    }
}

impl LintResult {
    pub fn new(raw: &str) -> Self {
        LintResult {
            line: 1,
            col: 1,
            filepath: String::from(""),
            raw: String::from(raw),
            lines: Vec::new(),
            error: String::from(""),
            enable: true,
        }
    }

    #[allow(dead_code)]
    pub fn to_json(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json) => json,
            _ => String::from("{}"),
        }
    }

    #[allow(dead_code)]
    pub fn to_json_pretty(&self) -> String {
        match serde_json::to_string_pretty(self) {
            Ok(json) => json,
            _ => String::from("{}"),
        }
    }

    #[allow(dead_code)]
    pub fn to_diff(&self) -> String {
        let mut out = String::from("");

        for line in self.lines.iter() {
            out.push_str(
                format!(
                    "{}:{}:{}\n",
                    self.filepath.replace("./", ""),
                    line.line,
                    line.col
                )
                .as_str(),
            );

            let out_str = crate::diff::diff_line_result(line);
            out.push_str(&out_str);
        }

        out
    }

    #[allow(dead_code)]
    pub fn has_error(&self) -> bool {
        !self.error.is_empty()
    }

    /// Return number of errors
    pub fn errors_count(&self) -> usize {
        self.lines.iter().filter(|l| l.severity.is_error()).count()
    }

    /// Return number of warnings
    pub fn warnings_count(&self) -> usize {
        self.lines
            .iter()
            .filter(|l| l.severity.is_warning())
            .count()
    }
}

impl Results for LintResult {
    fn push(&mut self, line_result: LineResult) {
        self.lines.push(line_result);
    }

    fn ignore(&mut self, part: &str) {
        // do nothing
        self.move_cursor(part);
    }

    fn error(&mut self, err: &str) {
        self.error = String::from(err);
    }

    fn to_string(&self) -> String {
        String::from("")
    }

    fn is_lint(&self) -> bool {
        true
    }

    fn toggle(&mut self, enable: bool) {
        self.enable = enable;
    }

    fn is_enabled(&self) -> bool {
        self.enable
    }

    /// Move the (line, col) with string part
    fn move_cursor(&mut self, part: &str) -> (usize, usize) {
        let (l, c, has_new_line) = line_col(part);

        let prev_line = self.line;
        let prev_col = self.col;

        self.line += l;
        if has_new_line {
            self.col = c;
        } else {
            self.col += c;
        }
        (prev_line, prev_col)
    }
}

/// Calculate line and col number of a string part
/// Fork from Pest for just count the part.
///
/// https://github.com/pest-parser/pest/blob/85b18aae23cc7b266c0b5252f9f74b7ab0000795/pest/src/position.rs#L135
fn line_col(part: &str) -> (usize, usize, bool) {
    let mut chars = part.chars().peekable();

    let mut line_col = (0, 0);
    let mut has_new_line = false;

    loop {
        match chars.next() {
            Some('\r') => {
                if let Some(&'\n') = chars.peek() {
                    chars.next();

                    line_col = (line_col.0 + 1, 1);
                    has_new_line = true;
                } else {
                    line_col = (line_col.0, line_col.1 + 1);
                }
            }
            Some('\n') => {
                line_col = (line_col.0 + 1, 1);
                has_new_line = true;
            }
            Some(_c) => {
                line_col = (line_col.0, line_col.1 + 1);
            }
            None => {
                break;
            }
        }
    }

    (line_col.0, line_col.1, has_new_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity() {
        assert_eq!(serde_json::to_string(&Severity::Error).unwrap(), "1");
        assert_eq!(serde_json::to_string(&Severity::Warning).unwrap(), "2");

        assert_eq!(Severity::Error.is_error(), true);
        assert_eq!(Severity::Error.is_warning(), false);
        assert_eq!(Severity::Warning.is_warning(), true);
        assert_eq!(Severity::Warning.is_error(), false);
    }

    #[test]
    fn test_move_cursor() {
        let mut out = LintResult::new("");
        assert_eq!((out.line, out.col), (1, 1));

        assert_eq!(out.move_cursor(""), (1, 1));
        assert_eq!((out.line, out.col), (1, 1));

        let raw = r#"Foo
Hello world
This is "#;
        assert_eq!(out.move_cursor(raw), (1, 1));
        assert_eq!((out.line, out.col), (3, 9));

        let raw = "Hello\nworld\r\nHello world\nHello world";
        assert_eq!(out.move_cursor(raw), (3, 9));
        assert_eq!((out.line, out.col), (6, 12));

        let raw = "Hello";
        assert_eq!(out.move_cursor(raw), (6, 12));
        assert_eq!((out.line, out.col), (6, 17));

        let raw = "\nHello\n\naaa\n";
        assert_eq!(out.move_cursor(raw), (6, 17));
        assert_eq!((out.line, out.col), (10, 1));
    }
}
