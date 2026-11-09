//! Converts the model's Markdown into the HTML subset Telegram understands
//! (<https://core.telegram.org/bots/api#html-style>). Constructs Telegram has no
//! equivalent for are approximated: headings become bold lines, list items get
//! bullets or numbers, table cells are separated with `|`, simple LaTeX symbols
//! become Unicode. Only absolute URLs become links, since Telegram rejects the
//! whole message otherwise.

use std::fmt::Write as _;

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

const CELL_SEPARATOR: &str = " | ";

pub fn to_telegram_html(markdown: &str) -> String {
    let options = Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TABLES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_MATH;
    let markdown = end_lists_before_unindented_lines(markdown);
    let mut renderer = Renderer::default();
    for event in Parser::new_ext(&markdown, options) {
        renderer.event(event);
    }
    renderer.out.trim().to_owned()
}

#[derive(Default)]
struct Renderer {
    out: String,
    /// Next number for each open list; `None` for bullet lists.
    lists: Vec<Option<u64>>,
    /// Whether the open code block was written as `<pre><code class=…>`.
    code_has_lang: bool,
    /// For each open link or image, whether an `<a>` tag was written for it.
    links: Vec<bool>,
}

impl Renderer {
    fn event(&mut self, event: Event) {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.out.push_str(&escape(&text));
            }
            Event::Code(code) => {
                let _ = write!(self.out, "<code>{}</code>", escape(&code));
            }
            Event::InlineMath(math) | Event::DisplayMath(math) => {
                self.out.push_str(&render_math(&math));
            }
            Event::SoftBreak | Event::HardBreak => self.out.push('\n'),
            Event::Rule => self.out.push_str("——————\n\n"),
            Event::TaskListMarker(done) => self.out.push_str(if done { "☑ " } else { "☐ " }),
            Event::FootnoteReference(_) => {}
        }
    }

    fn start(&mut self, tag: Tag) {
        let out = &mut self.out;
        match tag {
            Tag::Heading { level, .. } => {
                out.push_str(if is_major(level) { "<b><u>" } else { "<b>" });
            }
            Tag::BlockQuote(_) => out.push_str("<blockquote>"),
            Tag::CodeBlock(kind) => {
                let lang = match &kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next(),
                    CodeBlockKind::Indented => None,
                };
                self.code_has_lang = lang.is_some();
                match lang {
                    Some(lang) => {
                        let _ = write!(out, "<pre><code class=\"language-{}\">", escape(lang));
                    }
                    None => out.push_str("<pre>"),
                }
            }
            Tag::List(start) => {
                if !self.lists.is_empty() {
                    trim_trailing_newlines(out);
                    out.push('\n');
                }
                self.lists.push(start);
            }
            Tag::Item => {
                out.push_str(&"    ".repeat(self.lists.len().saturating_sub(1)));
                match self.lists.last_mut() {
                    Some(Some(n)) => {
                        let _ = write!(out, "{n}. ");
                        *n += 1;
                    }
                    _ => out.push_str("• "),
                }
            }
            Tag::Emphasis => out.push_str("<i>"),
            Tag::Strong | Tag::TableHead => out.push_str("<b>"),
            Tag::Strikethrough => out.push_str("<s>"),
            Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                let linkable = is_absolute_url(&dest_url);
                if linkable {
                    let _ = write!(out, "<a href=\"{}\">", escape(&dest_url));
                }
                self.links.push(linkable);
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        let out = &mut self.out;
        match tag {
            // Paragraphs inside list items only need a line break, others a blank line.
            TagEnd::Paragraph => out.push_str(if self.lists.is_empty() { "\n\n" } else { "\n" }),
            TagEnd::Heading(level) => {
                out.push_str(if is_major(level) {
                    "</u></b>\n\n"
                } else {
                    "</b>\n\n"
                });
            }
            TagEnd::BlockQuote(_) => {
                trim_trailing_newlines(out);
                out.push_str("</blockquote>\n\n");
            }
            TagEnd::CodeBlock => {
                trim_trailing_newlines(out);
                out.push_str(if self.code_has_lang {
                    "</code></pre>\n\n"
                } else {
                    "</pre>\n\n"
                });
            }
            TagEnd::List(_) => {
                self.lists.pop();
                if self.lists.is_empty() {
                    trim_trailing_newlines(out);
                    out.push_str("\n\n");
                }
            }
            TagEnd::Item => {
                trim_trailing_newlines(out);
                out.push('\n');
            }
            TagEnd::Emphasis => out.push_str("</i>"),
            TagEnd::Strong => out.push_str("</b>"),
            TagEnd::Strikethrough => out.push_str("</s>"),
            TagEnd::Link | TagEnd::Image => {
                if self.links.pop() == Some(true) {
                    out.push_str("</a>");
                }
            }
            TagEnd::TableHead => {
                end_table_row(out);
                out.push_str("</b>\n");
            }
            TagEnd::TableRow => {
                end_table_row(out);
                out.push('\n');
            }
            TagEnd::TableCell => out.push_str(CELL_SEPARATOR),
            TagEnd::Table => out.push('\n'),
            _ => {}
        }
    }
}

/// Models write Markdown as if every line stood alone, but per the Markdown spec an
/// unindented line right after a list item continues that item ("lazy
/// continuation"), so `**Next section:**` would end up glued to the last bullet.
/// A blank line before such lines ends the list as intended.
fn end_lists_before_unindented_lines(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len() + 16);
    let mut in_fence = false;
    let mut in_list = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            in_list = false;
        } else if !in_fence {
            let indented = trimmed.len() < line.len();
            if trimmed.is_empty() {
                in_list = false;
            } else if is_list_item(trimmed) {
                in_list = true;
            } else if in_list && !indented {
                out.push('\n');
                in_list = false;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

fn is_list_item(line: &str) -> bool {
    if ["* ", "- ", "+ "].iter().any(|m| line.starts_with(m)) {
        return true;
    }
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    digits > 0 && (line[digits..].starts_with(". ") || line[digits..].starts_with(") "))
}

/// Telegram rejects messages with links it can't open, such as relative paths.
fn is_absolute_url(url: &str) -> bool {
    ["http://", "https://", "tg://", "mailto:"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

/// LaTeX commands models use for simple symbols, longest first so that e.g.
/// `\leq` isn't eaten by `\le`.
const LATEX_SYMBOLS: &[(&str, &str)] = &[
    ("\\leftrightarrow", "↔"),
    ("\\Rightarrow", "⇒"),
    ("\\rightarrow", "→"),
    ("\\Leftarrow", "⇐"),
    ("\\leftarrow", "←"),
    ("\\approx", "≈"),
    ("\\degree", "°"),
    ("\\infty", "∞"),
    ("\\times", "×"),
    ("\\cdot", "·"),
    ("\\circ", "°"),
    ("\\div", "÷"),
    ("\\geq", "≥"),
    ("\\leq", "≤"),
    ("\\neq", "≠"),
    ("\\to", "→"),
    ("\\pm", "±"),
    ("\\ge", "≥"),
    ("\\le", "≤"),
    ("\\ne", "≠"),
];

/// `$\rightarrow$` becomes `→`; math without LaTeX commands (e.g. `$5-$`) keeps
/// its dollar signs; anything more complex is shown as code.
fn render_math(math: &str) -> String {
    if !math.contains('\\') {
        return format!("${}$", escape(math));
    }
    let replaced = LATEX_SYMBOLS
        .iter()
        .fold(math.trim().to_owned(), |text, (cmd, symbol)| {
            text.replace(cmd, symbol)
        });
    if replaced.contains('\\') {
        format!("<code>{}</code>", escape(math.trim()))
    } else {
        escape(&replaced)
    }
}

/// H1 and H2 are underlined as well as bold.
fn is_major(level: HeadingLevel) -> bool {
    level <= HeadingLevel::H2
}

fn end_table_row(out: &mut String) {
    if out.ends_with(CELL_SEPARATOR) {
        out.truncate(out.len() - CELL_SEPARATOR.len());
    }
}

fn trim_trailing_newlines(out: &mut String) {
    while out.ends_with('\n') {
        out.pop();
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::to_telegram_html as html;

    #[test]
    fn inline_formatting() {
        assert_eq!(
            html("**bold** *it* ~~gone~~ `x<y` [link](https://a.b/?q=1&r=2)"),
            "<b>bold</b> <i>it</i> <s>gone</s> <code>x&lt;y</code> \
             <a href=\"https://a.b/?q=1&amp;r=2\">link</a>"
        );
    }

    #[test]
    fn escapes_html() {
        assert_eq!(html("a < b & <script>"), "a &lt; b &amp; &lt;script&gt;");
    }

    #[test]
    fn code_blocks() {
        assert_eq!(
            html("`a` then\n```rust\nfn main() {}\n```"),
            "<code>a</code> then\n\n<pre><code class=\"language-rust\">fn main() {}</code></pre>"
        );
        assert_eq!(html("```\nplain\n```"), "<pre>plain</pre>");
    }

    #[test]
    fn headings_and_paragraphs() {
        assert_eq!(
            html("# Title\n\nFirst.\n\n### Sub\ntext"),
            "<b><u>Title</u></b>\n\nFirst.\n\n<b>Sub</b>\n\ntext"
        );
    }

    #[test]
    fn lists() {
        assert_eq!(
            html("- a\n- b\n  - c\n\nafter"),
            "• a\n• b\n    • c\n\nafter"
        );
        assert_eq!(html("3. x\n4. y"), "3. x\n4. y");
        assert_eq!(html("- [x] done\n- [ ] todo"), "• ☑ done\n• ☐ todo");
    }

    #[test]
    fn line_after_list_is_not_part_of_it() {
        assert_eq!(
            html("* a\n    * b\n**Next:**\n1. x"),
            "• a\n    • b\n\n<b>Next:</b>\n\n1. x"
        );
        // Indented continuation lines and fenced code are left alone.
        assert_eq!(html("* a\n  more"), "• a\nmore");
        assert_eq!(html("```\n* a\nb\n```"), "<pre>* a\nb</pre>");
    }

    #[test]
    fn only_absolute_urls_become_links() {
        assert_eq!(
            html("![alt](img/x.png) [rel](page) ![pic](https://x.y/a.png)"),
            "alt rel <a href=\"https://x.y/a.png\">pic</a>"
        );
    }

    #[test]
    fn math() {
        assert_eq!(html("a $\\rightarrow$ b $x \\leq 2$"), "a → b x ≤ 2");
        assert_eq!(html("$\\frac{1}{2}$"), "<code>\\frac{1}{2}</code>");
        assert_eq!(html("from $5-$10"), "from $5-$10");
    }

    #[test]
    fn table() {
        assert_eq!(
            html("| a | b |\n|---|---|\n| 1 | 2 |"),
            "<b>a | b</b>\n1 | 2"
        );
    }
}
