use std::path::Path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Slash,
    Css,
    Hash,
    Python,
    PowerShell,
    Html,
    Cmd,
}

impl Language {
    pub fn of(path: &str) -> Option<Self> {
        let extension = Path::new(path).extension()?.to_str()?.to_ascii_lowercase();
        Some(match extension.as_str() {
            "rs" => Self::Rust,
            "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs" | "c" | "h" | "cc"
            | "cpp" | "hpp" | "cs" | "java" | "go" | "gdshader" | "gdshaderinc" | "shader"
            | "glsl" | "vert" | "frag" | "wgsl" => Self::Slash,
            "css" | "scss" | "sass" => Self::Css,
            "gd" | "sh" | "bash" => Self::Hash,
            "py" | "pyi" => Self::Python,
            "ps1" | "psm1" | "psd1" => Self::PowerShell,
            "html" | "htm" | "xml" | "vue" | "svelte" => Self::Html,
            "bat" | "cmd" => Self::Cmd,
            _ => return None,
        })
    }

    pub fn block(self) -> Option<(&'static [u8], &'static [u8])> {
        match self {
            Self::Rust | Self::Slash | Self::Css => Some((b"/*", b"*/")),
            Self::PowerShell => Some((b"<#", b"#>")),
            Self::Html => Some((b"<!--", b"-->")),
            _ => None,
        }
    }

    pub fn line_comment(self, rest: &[u8]) -> bool {
        match self {
            Self::Rust | Self::Slash => rest.starts_with(b"//"),
            Self::Hash | Self::Python | Self::PowerShell => rest.starts_with(b"#"),
            _ => false,
        }
    }
}
