use front::{Diagnostic, Diagnostics, SourceFile, Span};

fn main() {
    let src = "fn café(x: i64) -> i64 {\n\tlet y = x + \"nope\";\n\ty\n}\n";
    let file = SourceFile::new("demo.lang", src);

    let mut diags = Diagnostics::new();
    diags.push(
        Diagnostic::error("mismatched types")
            .with_code("E0308")
            .with_secondary(Span::new(20, 23), "expected because of this return type")
            .with_label(Span::new(39, 45), "expected `i64`, found `str`")
            .with_note("no implicit conversion exists between `i64` and `str`"),
    );
    diags.push(
        Diagnostic::warning("unclosed block")
            .with_label(Span::new(0, 30), "this block starts here")
            .with_label(Span::new(src.len(), src.len()), "and the file ends here"),
    );

    print!("{}", diags.render(&file, true));
}
