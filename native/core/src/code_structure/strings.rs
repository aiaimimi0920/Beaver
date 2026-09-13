use super::language::Language;

pub struct Quoted {
    pub end: Vec<u8>,
    pub escape: Option<u8>,
    pub doubled: bool,
    pub line_start: bool,
    pub comment: bool,
}

impl Quoted {
    pub fn consume(&self, line: &[u8], position: usize) -> (usize, bool) {
        let rest = &line[position..];
        if self.escape.is_some_and(|escape| rest[0] == escape) {
            return ((position + 2).min(line.len()), false);
        }
        if rest.starts_with(&self.end) && (!self.line_start || position == 0) {
            let next = position + self.end.len();
            if self.doubled && line[next..].starts_with(&self.end) {
                return (next + self.end.len(), false);
            }
            return (next, true);
        }
        (position + 1, false)
    }
}

pub fn opening(
    line: &[u8],
    position: usize,
    language: Language,
    docstring: bool,
) -> Option<(Quoted, usize)> {
    let rest = &line[position..];
    let mut quote = Quoted {
        end: Vec::new(),
        escape: Some(b'\\'),
        doubled: false,
        line_start: false,
        comment: false,
    };
    if language == Language::Rust {
        let raw = if rest.starts_with(b"br") || rest.starts_with(b"cr") {
            2
        } else if rest.starts_with(b"r") {
            1
        } else {
            0
        };
        if raw > 0 {
            let hashes = rest[raw..].iter().take_while(|&&b| b == b'#').count();
            if rest.get(raw + hashes) == Some(&b'"') {
                quote.end = [vec![b'"'], vec![b'#'; hashes]].concat();
                quote.escape = None;
                return Some((quote, raw + hashes + 1));
            }
        }
        // A Rust lifetime/label is not the start of a multiline character literal.
        if rest[0] == b'\'' {
            let mut index = 1;
            if rest.get(index) == Some(&b'\\') {
                index += 2;
                if rest.get(2) == Some(&b'u') {
                    index = rest.iter().position(|&b| b == b'}').map(|i| i + 1)?;
                } else if rest.get(2) == Some(&b'x') {
                    index = 4;
                }
            } else {
                let value = std::str::from_utf8(&rest[1..]).ok()?.chars().next()?;
                index += value.len_utf8();
            }
            if rest.get(index) != Some(&b'\'') {
                return None;
            }
        }
    }
    if language == Language::PowerShell {
        quote.escape = Some(b'`');
        if rest.starts_with(b"@\"") || rest.starts_with(b"@'") {
            if rest[2..].iter().all(u8::is_ascii_whitespace) {
                quote.end = vec![rest[1], b'@'];
                quote.escape = None;
                quote.line_start = true;
                return Some((quote, 2));
            }
        }
        if rest[0] == b'\'' {
            quote.escape = None;
            quote.doubled = true;
        }
    }
    if matches!(language, Language::Hash | Language::Python)
        && (rest.starts_with(b"\"\"\"") || rest.starts_with(b"'''"))
    {
        quote.end = rest[..3].to_vec();
        quote.comment = language == Language::Python && docstring;
        return Some((quote, 3));
    }
    // C# verbatim strings preserve doubled quotes and backslashes.
    if language == Language::Slash && rest.starts_with(b"@\"") {
        quote.end = vec![b'"'];
        quote.escape = None;
        quote.doubled = true;
        return Some((quote, 2));
    }
    if matches!(rest[0], b'\'' | b'"' | b'`') {
        quote.end = vec![rest[0]];
        return Some((quote, 1));
    }
    None
}
