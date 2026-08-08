

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{IsTerminal, Write as _};
use std::ops::Range;

use crate::span::Span;


const TAB_WIDTH: usize = 4;

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const RED: &str = "\x1b[1;31m";
const YELLOW: &str = "\x1b[1;33m";
const CYAN: &str = "\x1b[1;36m";
const GREEN: &str = "\x1b[1;32m";
const BLUE: &str = "\x1b[1;34m";

/// Emits `code` only when coloring is on, so the same renderer produces both
/// terminal output and plain text suitable for test assertions.
fn c(color: bool, code: &'static str) -> &'static str {
    if color { code } else { "" }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
            Severity::Help => "help",
        }
    }

    fn color(self) -> &'static str {
        match self {
            Severity::Error => RED,
            Severity::Warning => YELLOW,
            Severity::Note => CYAN,
            Severity::Help => GREEN,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    /// The span the diagnostic is really about. Underlined with `^`.
    Primary,
    /// Supporting context elsewhere in the source. Underlined with `-`.
    Secondary,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub span: Span,
    pub message: Option<String>,
    pub style: LabelStyle,
}

impl Label {
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Label {
            span,
            message: Some(message.into()),
            style: LabelStyle::Primary,
        }
    }

    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Label {
            span,
            message: Some(message.into()),
            style: LabelStyle::Secondary,
        }
    }

    /// A bare underline with no text next to it.
    pub fn bare(span: Span) -> Self {
        Label {
            span,
            message: None,
            style: LabelStyle::Primary,
        }
    }

    fn underline(&self) -> char {
        match self.style {
            LabelStyle::Primary => '^',
            LabelStyle::Secondary => '-',
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub labels: Vec<Label>,
    /// Trailing `= note:` lines, for explanation that isn't tied to a span.
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn new(severity: Severity, message: impl Into<String>) -> Self {
        Diagnostic {
            severity,
            code: None,
            message: message.into(),
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(Severity::Error, message)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, message)
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::primary(span, message));
        self
    }

    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::secondary(span, message));
        self
    }

    pub fn with_bare_label(mut self, span: Span) -> Self {
        self.labels.push(Label::bare(span));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    /// The span this diagnostic points at: the first primary label, else the
    /// first label of any kind.
    pub fn primary_span(&self) -> Option<Span> {
        self.labels
            .iter()
            .find(|l| l.style == LabelStyle::Primary)
            .or_else(|| self.labels.first())
            .map(|l| l.span)
    }

    /// Renders the diagnostic as it should appear in a terminal. Always ends
    /// with a newline; never writes anywhere, so it can be asserted on.
    pub fn render(&self, file: &SourceFile, color: bool) -> String {
        let mut out = String::new();

        // error[E0001]: message
        let _ = write!(
            out,
            "{}{}",
            c(color, self.severity.color()),
            self.severity.as_str()
        );
        if let Some(code) = &self.code {
            let _ = write!(out, "[{code}]");
        }
        let _ = writeln!(
            out,
            "{}{}: {}{}",
            c(color, RESET),
            c(color, BOLD),
            self.message,
            c(color, RESET)
        );

        // Group labels by the line their start falls on, so a line is echoed
        // once with an underline row per label beneath it.
        let mut groups: BTreeMap<usize, Vec<&Label>> = BTreeMap::new();
        for label in &self.labels {
            let start = file.clamp(label.span.start);
            groups.entry(file.line_index(start)).or_default().push(label);
        }

        let gutter = groups
            .keys()
            .last()
            .map(|&line| (line + 1).to_string().len())
            .unwrap_or(1);
        let pad = " ".repeat(gutter);
        let bar = format!("{} {}|{}", pad, c(color, BLUE), c(color, RESET));

        if let Some(span) = self.primary_span() {
            let loc = file.location(span.start);
            let _ = writeln!(
                out,
                "{}{}-->{} {}:{}:{}",
                pad,
                c(color, BLUE),
                c(color, RESET),
                file.name(),
                loc.line,
                loc.column
            );
        }

        if !groups.is_empty() {
            let _ = writeln!(out, "{bar}");
        }

        let mut prev_line: Option<usize> = None;
        for (line, labels) in &groups {
            // Non-adjacent snippets get an elision marker rather than silently
            // running together.
            if let Some(prev) = prev_line
                && line - prev > 1
            {
                let _ = writeln!(out, "{}...{}", c(color, BLUE), c(color, RESET));
            }
            prev_line = Some(*line);

            let range = file.line_range(*line);
            let text = &file.source()[range.clone()];
            let (expanded, cols) = expand_tabs(text);

            let _ = writeln!(
                out,
                "{}{:>width$}{} {}|{} {}",
                c(color, BLUE),
                line + 1,
                c(color, RESET),
                c(color, BLUE),
                c(color, RESET),
                expanded,
                width = gutter
            );

            let mut labels = labels.clone();
            labels.sort_by_key(|l| l.span.start);
            for label in labels {
                let (col, width) = underline_extent(file, label.span, &range, &cols);
                let color_code = match label.style {
                    LabelStyle::Primary => self.severity.color(),
                    LabelStyle::Secondary => BLUE,
                };
                let _ = write!(
                    out,
                    "{} {}{}{}",
                    bar,
                    " ".repeat(col),
                    c(color, color_code),
                    label.underline().to_string().repeat(width)
                );
                match &label.message {
                    Some(msg) => {
                        let _ = writeln!(out, " {}{}", msg, c(color, RESET));
                    }
                    None => {
                        let _ = writeln!(out, "{}", c(color, RESET));
                    }
                }
            }
        }

        if !self.notes.is_empty() {
            if !groups.is_empty() {
                let _ = writeln!(out, "{bar}");
            }
            for note in &self.notes {
                let _ = writeln!(
                    out,
                    "{} {}={} {}note:{} {}",
                    pad,
                    c(color, BLUE),
                    c(color, RESET),
                    c(color, BOLD),
                    c(color, RESET),
                    note
                );
            }
        }

        out
    }

    /// Renders to stderr, coloring only when stderr is a terminal.
    pub fn emit(&self, file: &SourceFile) {
        let text = self.render(file, use_color());
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "{text}");
    }
}

/// Where an underline sits on a rendered line: starting display column and
/// width in columns. Spans reaching past the line are clipped to it, and a
/// zero-width span still gets one caret.
fn underline_extent(
    file: &SourceFile,
    span: Span,
    range: &Range<usize>,
    cols: &[usize],
) -> (usize, usize) {
    let start = file.clamp(span.start).clamp(range.start, range.end);
    let end = file.clamp(span.end).clamp(start, range.end);
    let col_start = cols[start - range.start];
    let col_end = cols[end - range.start];
    (col_start, (col_end - col_start).max(1))
}

/// Expands tabs to [`TAB_WIDTH`] stops and returns, alongside the expanded
/// text, a byte-offset → display-column map with one entry per byte plus a
/// final entry for the end of the line. Interior bytes of a multi-byte
/// character map to that character's column, so a span landing mid-character
/// still resolves somewhere sane.
fn expand_tabs(line: &str) -> (String, Vec<usize>) {
    let mut out = String::with_capacity(line.len());
    let mut cols = vec![0usize; line.len() + 1];
    let mut col = 0usize;

    for (i, ch) in line.char_indices() {
        cols[i..i + ch.len_utf8()].fill(col);
        if ch == '\t' {
            let next = (col / TAB_WIDTH + 1) * TAB_WIDTH;
            for _ in col..next {
                out.push(' ');
            }
            col = next;
        } else {
            out.push(ch);
            col += 1;
        }
    }
    cols[line.len()] = col;

    (out, cols)
}

fn use_color() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}

/// 1-based line and column, counted in characters — what an editor would show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

/// A named source string with a precomputed line table.
#[derive(Debug, Clone)]
pub struct SourceFile {
    name: String,
    src: String,
    /// Byte offset of the first character of each line.
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, src: impl Into<String>) -> Self {
        let src = src.into();
        let mut line_starts = vec![0];
        line_starts.extend(
            src.char_indices()
                .filter(|&(_, ch)| ch == '\n')
                .map(|(i, _)| i + 1),
        );
        // A file ending in a newline would otherwise gain an empty final line,
        // and every unexpected-EOF error — the most common parser error there
        // is — would point a caret at that blank line instead of at the last
        // line with code on it.
        if src.ends_with('\n') {
            line_starts.pop();
        }
        SourceFile {
            name: name.into(),
            src,
            line_starts,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.src
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Clamps an offset into the file and back onto a character boundary, so a
    /// bad span degrades into a misplaced caret instead of a panic.
    fn clamp(&self, offset: usize) -> usize {
        let mut offset = offset.min(self.src.len());
        while offset > 0 && !self.src.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }

    /// 0-based index of the line containing `offset`.
    pub fn line_index(&self, offset: usize) -> usize {
        self.line_starts
            .partition_point(|&start| start <= offset)
            .saturating_sub(1)
    }

    /// Byte range of a line's text, excluding its `\r\n` or `\n` terminator.
    /// Keeping `\r` out matters: echoed into a terminal it would return the
    /// cursor to column 0 and the underline row would overwrite the source.
    pub fn line_range(&self, line: usize) -> Range<usize> {
        let start = self.line_starts.get(line).copied().unwrap_or(self.src.len());
        let mut end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.src.len());
        let bytes = self.src.as_bytes();
        if end > start && bytes[end - 1] == b'\n' {
            end -= 1;
        }
        if end > start && bytes[end - 1] == b'\r' {
            end -= 1;
        }
        start..end
    }

    pub fn line(&self, line: usize) -> &str {
        &self.src[self.line_range(line)]
    }

    pub fn location(&self, offset: usize) -> Location {
        let offset = self.clamp(offset);
        let line = self.line_index(offset);
        let range = self.line_range(line);
        let end = offset.clamp(range.start, range.end);
        Location {
            line: line + 1,
            column: self.src[range.start..end].chars().count() + 1,
        }
    }
}

/// Accumulates diagnostics so a pass can report several problems in one run
/// instead of stopping at the first.
#[derive(Debug, Clone, Default)]
pub struct Diagnostics {
    items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, Diagnostic> {
        self.items.iter()
    }

    pub fn has_errors(&self) -> bool {
        self.items.iter().any(|d| d.severity == Severity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count()
    }

    pub fn render(&self, file: &SourceFile, color: bool) -> String {
        let mut out = String::new();
        for (i, diagnostic) in self.items.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&diagnostic.render(file, color));
        }
        out
    }

    pub fn emit(&self, file: &SourceFile) {
        let text = self.render(file, use_color());
        let mut err = std::io::stderr().lock();
        let _ = write!(err, "{text}");
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_a_labelled_error_with_a_note() {
        let file = SourceFile::new("main.lang", "let x = 5;\nlet y = x + z;\n");
        let rendered = Diagnostic::error("cannot find value `z` in this scope")
            .with_code("E0425")
            .with_label(Span::new(23, 24), "not found in this scope")
            .with_note("consider declaring `z` with `let`")
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
error[E0425]: cannot find value `z` in this scope
 --> main.lang:2:13
  |
2 | let y = x + z;
  |             ^ not found in this scope
  |
  = note: consider declaring `z` with `let`
"
        );
    }

    /// A tab is one byte but four columns, and `é`/`ø` are two bytes but one
    /// column each — the underline has to track columns, not bytes.
    #[test]
    fn aligns_underlines_past_tabs_and_multibyte_characters() {
        let file = SourceFile::new("t.lang", "\tlet café = ø;");
        let rendered = Diagnostic::error("unexpected character")
            .with_bare_label(Span::new(13, 15))
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
error: unexpected character
 --> t.lang:1:13
  |
1 |     let café = ø;
  |                ^
"
        );
    }

    /// A `\r` echoed into a terminal returns the cursor to column 0, so the
    /// underline row would paint over the source line. Captured test output
    /// hides this, hence the explicit assertion.
    #[test]
    fn strips_carriage_returns_from_echoed_lines() {
        let file = SourceFile::new("t.lang", "let x = 1;\r\nlet y = 2;\r\n");
        let rendered = Diagnostic::error("second line")
            .with_label(Span::new(16, 17), "here")
            .render(&file, false);

        assert!(!rendered.contains('\r'), "rendered output kept a \\r");
        assert_eq!(
            rendered,
            "\
error: second line
 --> t.lang:2:5
  |
2 | let y = 2;
  |     ^ here
"
        );
    }

    /// The lexer already emits `Span { start: n, end: n }` for EOF.
    #[test]
    fn zero_width_span_still_gets_one_caret() {
        let file = SourceFile::new("t.lang", "let x =");
        let rendered = Diagnostic::error("expected expression")
            .with_label(Span::at(file.source().len()), "expected an expression here")
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
error: expected expression
 --> t.lang:1:8
  |
1 | let x =
  |        ^ expected an expression here
"
        );
    }

    /// v0 policy: a span crossing lines is clipped to the line it starts on.
    #[test]
    fn multiline_span_clips_to_its_first_line() {
        let file = SourceFile::new("t.lang", "fn f() {\n  1\n");
        let rendered = Diagnostic::error("unclosed block")
            .with_label(Span::new(0, 13), "this block is never closed")
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
error: unclosed block
 --> t.lang:1:1
  |
1 | fn f() {
  | ^^^^^^^^ this block is never closed
"
        );
    }

    #[test]
    fn elides_the_gap_between_distant_labels() {
        let file = SourceFile::new("t.lang", "a\nb\nc\nd\ne\n");
        let rendered = Diagnostic::error("two things")
            .with_label(Span::new(0, 1), "here")
            .with_secondary(Span::new(6, 7), "and here")
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
error: two things
 --> t.lang:1:1
  |
1 | a
  | ^ here
...
4 | d
  | - and here
"
        );
    }

    #[test]
    fn shares_one_source_line_between_labels_on_it() {
        let file = SourceFile::new("t.lang", "let x: i64 = \"hi\";\n");
        let rendered = Diagnostic::error("mismatched types")
            .with_secondary(Span::new(7, 10), "expected because of this")
            .with_label(Span::new(13, 17), "expected `i64`, found `str`")
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
error: mismatched types
 --> t.lang:1:14
  |
1 | let x: i64 = \"hi\";
  |        --- expected because of this
  |              ^^^^ expected `i64`, found `str`
"
        );
    }

    #[test]
    fn gutter_widens_for_larger_line_numbers() {
        let src = "x\n".repeat(120);
        let file = SourceFile::new("t.lang", src);
        let rendered = Diagnostic::warning("late line")
            .with_label(Span::new(200, 201), "here")
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
warning: late line
   --> t.lang:101:1
    |
101 | x
    | ^ here
"
        );
    }

    #[test]
    fn color_is_opt_in() {
        let file = SourceFile::new("t.lang", "x\n");
        let d = Diagnostic::error("boom").with_label(Span::new(0, 1), "here");

        assert!(!d.render(&file, false).contains('\x1b'));
        assert!(d.render(&file, true).contains('\x1b'));
    }

    #[test]
    fn collector_tracks_errors_separately_from_warnings() {
        let file = SourceFile::new("t.lang", "a\n");
        let mut diags = Diagnostics::new();
        assert!(!diags.has_errors());

        diags.push(Diagnostic::warning("careful").with_label(Span::new(0, 1), "hm"));
        assert!(!diags.has_errors());

        diags.push(Diagnostic::error("nope").with_label(Span::new(0, 1), "hm"));
        assert!(diags.has_errors());
        assert_eq!(diags.error_count(), 1);
        assert_eq!(diags.len(), 2);

        // Blank line between diagnostics, none trailing.
        let rendered = diags.render(&file, false);
        assert!(rendered.contains("hm\n\nerror: nope"));
        assert!(rendered.ends_with("hm\n"));
    }

    #[test]
    fn out_of_range_and_reversed_spans_do_not_panic() {
        let file = SourceFile::new("t.lang", "héllo\n");
        for span in [
            Span::new(9_000, 9_100),
            Span::new(4, 2),
            Span::new(2, 2),
            // Mid-character offsets: `é` occupies bytes 1..3.
            Span::new(2, 3),
        ] {
            let _ = Diagnostic::error("x").with_bare_label(span).render(&file, false);
        }
    }

    /// A trailing newline must not create a blank final line for EOF errors to
    /// point at; the caret belongs just past the last line with code on it.
    #[test]
    fn eof_in_a_newline_terminated_file_points_at_the_last_real_line() {
        let src = "fn f() {\n  1\n";
        let file = SourceFile::new("t.lang", src);
        assert_eq!(file.line_count(), 2);

        let rendered = Diagnostic::error("unexpected end of file")
            .with_label(Span::at(src.len()), "expected `}`")
            .render(&file, false);

        assert_eq!(
            rendered,
            "\
error: unexpected end of file
 --> t.lang:2:4
  |
2 |   1
  |    ^ expected `}`
"
        );
    }

    #[test]
    fn locations_count_characters_not_bytes() {
        let file = SourceFile::new("t.lang", "héllo\nwörld\n");
        assert_eq!(file.location(0), Location { line: 1, column: 1 });
        // `o` in héllo: bytes h=0, é=1..3, l=3, l=4, o=5.
        assert_eq!(file.location(5), Location { line: 1, column: 5 });
        assert_eq!(file.line(1), "wörld");
        assert_eq!(file.line_index(7), 1);
    }
}
