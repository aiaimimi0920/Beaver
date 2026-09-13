use super::{
    language::Language,
    strings::{opening, Quoted},
};

pub fn count(source: &str, language: Language) -> usize {
    let mut block_depth = 0;
    let mut quoted: Option<Quoted> = None;
    let mut effective = 0;
    let mut python_docstring = true;
    for raw in source.trim_start_matches('\u{feff}').split('\n') {
        let line = raw.as_bytes();
        let trimmed = raw.trim_start();
        if quoted.is_none() && block_depth == 0 && language == Language::Cmd {
            let command = trimmed.strip_prefix('@').unwrap_or(trimmed);
            let first = command.split_whitespace().next().unwrap_or("");
            if command.starts_with("::") || first.eq_ignore_ascii_case("rem") {
                continue;
            }
        }
        let mut position = 0;
        let mut has_code = false;
        let mut had_docstring = false;
        while position < line.len() {
            let rest = &line[position..];
            if let Some(current) = &quoted {
                has_code |= !current.comment && !rest[0].is_ascii_whitespace();
                let (next, closed) = current.consume(line, position);
                position = next;
                if closed {
                    quoted = None;
                }
                continue;
            }
            if let Some((start, end)) = language.block() {
                if block_depth > 0 {
                    if rest.starts_with(end) {
                        block_depth -= 1;
                        position += end.len();
                    } else if language == Language::Rust && rest.starts_with(start) {
                        block_depth += 1;
                        position += start.len();
                    } else {
                        position += 1;
                    }
                    continue;
                }
                if rest.starts_with(start) {
                    block_depth = 1;
                    position += start.len();
                    continue;
                }
            }
            if language.line_comment(rest) {
                break;
            }
            if rest[0].is_ascii_whitespace() {
                position += 1;
                continue;
            }
            if let Some((value, length)) =
                opening(line, position, language, python_docstring && !has_code)
            {
                has_code |= !value.comment;
                had_docstring |= value.comment;
                quoted = Some(value);
                position += length;
            } else {
                has_code = true;
                position += 1;
            }
        }
        effective += usize::from(has_code);
        if language == Language::Python && (has_code || had_docstring) {
            // Only the first statement in a module/class/function is a docstring.
            python_docstring = quoted.is_none()
                && trimmed.ends_with(':')
                && ["def ", "async def ", "class "]
                    .iter()
                    .any(|s| trimmed.starts_with(s));
        }
    }
    effective
}
