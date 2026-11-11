//! A quick outline of the open file for "Go to Symbol in File": functions, types, headings and
//! sections found by simple per-language line patterns. There is no language server behind it, so it
//! is a convenience, not a precise index: it finds what is written at the start of a line.

use fancy_regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Symbol {
    pub name: String,
    /// What it is, for the label next to the name (`fn`, `class`, `heading`...).
    pub kind: &'static str,
    /// 1-based line of its start.
    pub line: usize,
}

/// One pattern: the kind it finds, and a regex whose first capture group is the name.
struct Rule {
    kind: &'static str,
    regex: Regex,
}

fn rules(patterns: &[(&'static str, &str)]) -> Vec<Rule> {
    patterns
        .iter()
        .map(|(kind, pattern)| Rule {
            kind,
            regex: Regex::new(pattern).expect("a symbol pattern is a valid regex"),
        })
        .collect()
}

#[derive(Clone, Copy)]
enum Family {
    Rust,
    Python,
    Script,
    Go,
    Classes,
    Markdown,
    Shell,
    Sections,
}

fn family(extension: &str) -> Option<Family> {
    Some(match extension.to_ascii_lowercase().as_str() {
        "rs" => Family::Rust,
        "py" | "pyw" => Family::Python,
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" => Family::Script,
        "go" => Family::Go,
        "java" | "cs" | "kt" | "swift" | "dart" | "php" => Family::Classes,
        "md" | "markdown" => Family::Markdown,
        "sh" | "bash" | "zsh" => Family::Shell,
        "toml" | "ini" | "cfg" | "conf" => Family::Sections,
        _ => return None,
    })
}

fn rules_for(family: Family) -> &'static [Rule] {
    static RUST: OnceLock<Vec<Rule>> = OnceLock::new();
    static PYTHON: OnceLock<Vec<Rule>> = OnceLock::new();
    static SCRIPT: OnceLock<Vec<Rule>> = OnceLock::new();
    static GO: OnceLock<Vec<Rule>> = OnceLock::new();
    static CLASSES: OnceLock<Vec<Rule>> = OnceLock::new();
    static MARKDOWN: OnceLock<Vec<Rule>> = OnceLock::new();
    static SHELL: OnceLock<Vec<Rule>> = OnceLock::new();
    static SECTIONS: OnceLock<Vec<Rule>> = OnceLock::new();
    // Visibility, qualifiers and modifiers that may come before the keyword.
    const RUST_PREFIX: &str = r#"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:(?:async|unsafe|const|default)\s+)*(?:extern\s+"[^"]*"\s+)?"#;
    match family {
        Family::Rust => RUST.get_or_init(|| {
            rules(&[
                ("fn", &format!(r"{RUST_PREFIX}fn\s+([A-Za-z_]\w*)")),
                ("struct", &format!(r"{RUST_PREFIX}struct\s+([A-Za-z_]\w*)")),
                ("enum", &format!(r"{RUST_PREFIX}enum\s+([A-Za-z_]\w*)")),
                ("trait", &format!(r"{RUST_PREFIX}trait\s+([A-Za-z_]\w*)")),
                ("impl", r"^\s*(?:unsafe\s+)?impl\b\s*(?:<[^>]*>\s*)?([^{]+?)\s*(?:\{|where\b|$)"),
                ("mod", &format!(r"{RUST_PREFIX}mod\s+([A-Za-z_]\w*)")),
                ("type", &format!(r"{RUST_PREFIX}type\s+([A-Za-z_]\w*)")),
                ("macro", r"^\s*macro_rules!\s*([A-Za-z_]\w*)"),
            ])
        }),
        Family::Python => PYTHON.get_or_init(|| {
            rules(&[
                ("def", r"^\s*(?:async\s+)?def\s+([A-Za-z_]\w*)"),
                ("class", r"^\s*class\s+([A-Za-z_]\w*)"),
            ])
        }),
        Family::Script => SCRIPT.get_or_init(|| {
            rules(&[
                (
                    "function",
                    r"^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s*\*?\s*([A-Za-z_$][\w$]*)",
                ),
                (
                    "class",
                    r"^\s*(?:export\s+)?(?:default\s+)?(?:abstract\s+)?class\s+([A-Za-z_$][\w$]*)",
                ),
                ("interface", r"^\s*(?:export\s+)?interface\s+([A-Za-z_$][\w$]*)"),
                ("type", r"^\s*(?:export\s+)?type\s+([A-Za-z_$][\w$]*)\s*(?:<[^>]*>)?\s*="),
                ("enum", r"^\s*(?:export\s+)?(?:const\s+)?enum\s+([A-Za-z_$][\w$]*)"),
                (
                    "function",
                    r"^\s*(?:export\s+)?(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*(?::[^=]+)?=\s*(?:async\s*)?(?:\([^)]*\)|[A-Za-z_$][\w$]*)\s*(?::[^=]+)?=>",
                ),
            ])
        }),
        Family::Go => GO.get_or_init(|| {
            rules(&[
                ("func", r"^func\s+(?:\([^)]*\)\s*)?([A-Za-z_]\w*)"),
                ("type", r"^type\s+([A-Za-z_]\w*)"),
            ])
        }),
        Family::Classes => CLASSES.get_or_init(|| {
            rules(&[(
                "class",
                r"^\s*(?:[A-Za-z]+\s+)*?(?:class|interface|enum|record|struct|object|trait)\s+([A-Za-z_]\w*)",
            )])
        }),
        Family::Markdown => MARKDOWN.get_or_init(|| rules(&[("heading", r"^\s{0,3}#{1,6}\s+(.+?)\s*#*\s*$")])),
        Family::Shell => SHELL.get_or_init(|| {
            rules(&[
                ("function", r"^\s*function\s+([A-Za-z_][\w-]*)"),
                ("function", r"^\s*([A-Za-z_][\w-]*)\s*\(\)\s*\{?"),
            ])
        }),
        Family::Sections => SECTIONS.get_or_init(|| rules(&[("section", r"^\s*\[+\s*([^\]\s][^\]]*?)\s*\]+\s*(?:#.*)?$")])),
    }
}

/// The symbols of `text`, in file order. Nothing for file types without patterns.
pub fn outline(text: &str, extension: &str) -> Vec<Symbol> {
    let Some(family) = family(extension) else {
        return Vec::new();
    };
    let rules = rules_for(family);
    let mut found = Vec::new();
    let mut in_fence = false;
    for (index, line) in text.lines().enumerate() {
        // Lines of a code sample inside a Markdown fence are not headings.
        if matches!(family, Family::Markdown) && line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || line.len() > 400 {
            continue;
        }
        for rule in rules {
            // A pattern that gives up (a runaway match) simply finds nothing on that line.
            if let Ok(Some(captures)) = rule.regex.captures(line)
                && let Some(name) = captures.get(1)
            {
                found.push(Symbol {
                    name: name.as_str().trim().to_string(),
                    kind: rule.kind,
                    line: index + 1,
                });
                break;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(text: &str, extension: &str) -> Vec<(String, &'static str, usize)> {
        outline(text, extension)
            .into_iter()
            .map(|symbol| (symbol.name, symbol.kind, symbol.line))
            .collect()
    }

    #[test]
    fn rust_items_are_found_with_their_qualifiers() {
        let text = "\
use std::fmt;
pub struct Point { x: i32 }
pub(crate) async fn run() {}
impl<T> Display for Point<T> where T: Copy {
    fn fmt(&self) {}
}
unsafe impl Send for Point {}
mod tests {}
enum Kind { A }
pub trait Shape {}
type Alias = u8;
const X: u8 = 1;
macro_rules! hello { () => {} }
";
        assert_eq!(
            names(text, "rs"),
            vec![
                ("Point".into(), "struct", 2),
                ("run".into(), "fn", 3),
                ("Display for Point<T>".into(), "impl", 4),
                ("fmt".into(), "fn", 5),
                ("Send for Point".into(), "impl", 7),
                ("tests".into(), "mod", 8),
                ("Kind".into(), "enum", 9),
                ("Shape".into(), "trait", 10),
                ("Alias".into(), "type", 11),
                ("hello".into(), "macro", 13),
            ]
        );
    }

    #[test]
    fn script_python_and_go_declarations_are_found() {
        let ts = "\
export default async function load() {}
export class Store extends Base {}
export interface Options {}
type Id = string;
const add = (a: number, b: number): number => a + b;
let later = async x => x;
const value = 3;
";
        assert_eq!(
            names(ts, "ts"),
            vec![
                ("load".into(), "function", 1),
                ("Store".into(), "class", 2),
                ("Options".into(), "interface", 3),
                ("Id".into(), "type", 4),
                ("add".into(), "function", 5),
                ("later".into(), "function", 6),
            ]
        );
        let py = "import os\nclass A:\n    async def go(self): pass\ndef main(): pass\nx = 1\n";
        assert_eq!(
            names(py, "py"),
            vec![
                ("A".into(), "class", 2),
                ("go".into(), "def", 3),
                ("main".into(), "def", 4)
            ]
        );
        let go = "package x\nfunc main() {}\nfunc (s *Server) Run() {}\ntype Server struct {}\n";
        assert_eq!(
            names(go, "go"),
            vec![
                ("main".into(), "func", 2),
                ("Run".into(), "func", 3),
                ("Server".into(), "type", 4)
            ]
        );
    }

    #[test]
    fn markdown_headings_skip_code_fences_and_sections_and_shell_functions_work() {
        let md = "# Title\ntext\n```\n# not a heading\n```\n## Second ##\n";
        assert_eq!(
            names(md, "md"),
            vec![
                ("Title".into(), "heading", 1),
                ("Second".into(), "heading", 6)
            ]
        );
        let toml = "[package]\nname = \"x\"\n[dependencies.serde]\n";
        assert_eq!(
            names(toml, "toml"),
            vec![
                ("package".into(), "section", 1),
                ("dependencies.serde".into(), "section", 3)
            ]
        );
        let sh = "function build { :; }\ndeploy() {\n  echo\n}\n";
        assert_eq!(
            names(sh, "sh"),
            vec![
                ("build".into(), "function", 1),
                ("deploy".into(), "function", 2)
            ]
        );
        let java = "public final class Main {\n  static enum Mode { A }\n}\n";
        assert_eq!(
            names(java, "java"),
            vec![("Main".into(), "class", 1), ("Mode".into(), "class", 2)]
        );
    }

    #[test]
    fn unknown_types_empty_text_and_huge_lines_give_nothing_or_skip() {
        assert!(outline("fn main() {}", "txt").is_empty());
        assert!(outline("", "rs").is_empty());
        let long = format!("fn {}() {{}}", "a".repeat(500));
        assert!(
            outline(&long, "rs").is_empty(),
            "a minified line is skipped"
        );
        // A line that merely mentions a keyword is not a declaration.
        assert!(outline("// fn not_real() {}\nlet s = \"fn nope\";", "rs").is_empty());
        // The extension is matched without regard to case.
        assert_eq!(outline("def f(): pass", "PY").len(), 1);
    }
}
