// Cybermanju Drive — Tree-sitter Code Intelligence Module
// Incremental parsing for 200+ languages
// Extracts symbols, structure, semantic info for file organization
//
// In production: uses the `tree-sitter` crate with language grammars.
// For now, heuristic regex-based extraction provides the same JSON shape.

use anyhow::Result;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Language detection by file extension
// ---------------------------------------------------------------------------

/// Map a file extension to a language name.
pub fn detect_language(filename: &str) -> &'static str {
    let ext = filename.rsplit('.').next().unwrap_or("");
    match ext {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "hpp" | "cc" | "cxx" => "cpp",
        "java" => "java",
        "rb" => "ruby",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "html" | "htm" => "html",
        "css" | "scss" | "less" => "css",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "md" | "mdx" => "markdown",
        "sql" => "sql",
        "sh" | "bash" | "zsh" => "bash",
        "lua" => "lua",
        "zig" => "zig",
        "ex" | "exs" => "elixir",
        "vue" => "vue",
        "svelte" => "svelte",
        "dart" => "dart",
        "r" => "r",
        "scala" => "scala",
        "hs" => "haskell",
        "clj" | "cljs" => "clojure",
        "pl" | "pm" => "perl",
        "php" => "php",
        "cs" => "csharp",
        "fs" | "fsi" => "fsharp",
        "vb" => "visualbasic",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "cmake" => "cmake",
        _ => "unknown",
    }
}

// ---------------------------------------------------------------------------
// Heuristic symbol extraction
// ---------------------------------------------------------------------------

/// Extract symbols from source code using heuristic line-by-line scanning.
///
/// In production, this is replaced by tree-sitter query API:
///   ```
///   let mut parser = Parser::new();
///   parser.set_language(&LANGUAGE_RUST)?;
///   let tree = parser.parse(content, None)?;
///   let query = Query::new(&LANGUAGE_RUST, "(function_item name: (_) @name)")?;
///   ```
fn extract_symbols_heuristic(content: &str, language: &str) -> Vec<Value> {
    let mut symbols = Vec::new();
    let mut line_num: u32 = 0;

    // Language-aware keyword sets
    let fn_keywords = match language {
        "rust" => vec![
            "fn ",
            "pub fn ",
            "pub(crate) fn ",
            "pub(super) fn ",
            "async fn ",
            "pub async fn ",
        ],
        "python" => vec!["def "],
        "javascript" | "typescript" => vec![
            "function ",
            "const ",
            "let ",
            "var ",
            "class ",
            "async function ",
            "export function ",
            "export default function ",
        ],
        "go" => vec!["func "],
        "java" | "kotlin" | "csharp" => vec![
            "public ",
            "private ",
            "protected ",
            "static ",
            "class ",
            "interface ",
            "enum ",
            "void ",
        ],
        "c" | "cpp" => vec![
            "void ", "int ", "float ", "double ", "char ", "bool ", "auto ", "class ", "struct ",
            "enum ", "typedef ",
        ],
        "ruby" => vec!["def ", "class ", "module "],
        "swift" => vec!["func ", "class ", "struct ", "enum ", "protocol "],
        _ => vec![
            "fn ",
            "function ",
            "def ",
            "class ",
            "struct ",
            "interface ",
            "trait ",
            "impl ",
            "enum ",
            "type ",
        ],
    };

    for line in content.lines() {
        line_num += 1;
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("/*")
        {
            continue;
        }

        // Check for function definitions
        for kw in &fn_keywords {
            if trimmed.starts_with(kw) {
                let rest = &trimmed[kw.len()..];
                let name = rest
                    .split(|c: char| c == '(' || c == '<' || c == ':' || c == '{' || c == ' ')
                    .next()
                    .unwrap_or("")
                    .trim();

                if !name.is_empty() {
                    // Determine if this is a function, class, etc.
                    let kind = if trimmed.contains("class ")
                        || trimmed.contains("struct ")
                        || trimmed.contains("enum ")
                        || trimmed.contains("interface ")
                        || trimmed.contains("trait ")
                        || trimmed.contains("protocol ")
                        || trimmed.contains("module ")
                    {
                        "class"
                    } else if trimmed.contains("impl ") {
                        "impl"
                    } else {
                        "function"
                    };

                    symbols.push(json!({
                        "name": name,
                        "kind": kind,
                        "start_line": line_num,
                        "end_line": line_num,
                        "detail": trimmed,
                        "children": [],
                    }));
                }
                break;
            }
        }

        // Rust-specific: pub trait, pub struct, pub enum
        if language == "rust" {
            if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                let name = trimmed
                    .split_whitespace()
                    .nth(if trimmed.starts_with("pub trait ") {
                        2
                    } else {
                        1
                    })
                    .and_then(|n| n.split('<').next())
                    .unwrap_or("anonymous")
                    .split('{')
                    .next()
                    .unwrap_or("anonymous")
                    .trim();
                symbols.push(json!({
                    "name": name,
                    "kind": "interface",
                    "start_line": line_num,
                    "end_line": line_num,
                    "detail": trimmed,
                    "children": [],
                }));
            }
        }
    }

    symbols
}

// ---------------------------------------------------------------------------
// Tauri commands — registered directly in lib.rs invoke_handler
// ---------------------------------------------------------------------------

/// Parse a source file and return a structured JSON result with language,
/// symbols, line count, and timing.
///
/// Returns:
/// ```json
/// {
///   "file_path": "...",
///   "language": "rust",
///   "symbols": [...],
///   "total_lines": 42,
///   "parse_time_ms": 1
/// }
/// ```
#[tauri::command]
pub fn parse_file(file_path: String) -> Result<Value, String> {
    let start = std::time::Instant::now();

    // Read the file from disk
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;

    // Extract filename for language detection
    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let language = detect_language(filename);
    let total_lines = content.lines().count();
    let symbols = extract_symbols_heuristic(&content, language);
    let parse_time_ms = start.elapsed().as_millis() as u64;

    let result = json!({
        "file_path": file_path,
        "language": language,
        "symbols": symbols,
        "total_lines": total_lines,
        "parse_time_ms": parse_time_ms,
    });

    Ok(result)
}

/// Parse a source file and return just the symbols array as JSON values.
#[tauri::command]
pub fn get_symbols(file_path: String) -> Result<Vec<Value>, String> {
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read file {}: {}", file_path, e))?;

    let filename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    let language = detect_language(filename);
    let symbols = extract_symbols_heuristic(&content, language);

    Ok(symbols)
}
