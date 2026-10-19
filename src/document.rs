//! Buffer de texto, detecção/preservação de encoding e fim de linha.

use std::fmt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Windows1252,
}

impl Encoding {
    pub const ALL: [Encoding; 5] = [
        Encoding::Utf8,
        Encoding::Utf8Bom,
        Encoding::Utf16Le,
        Encoding::Utf16Be,
        Encoding::Windows1252,
    ];
}

impl fmt::Display for Encoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Encoding::Utf8 => "UTF-8",
            Encoding::Utf8Bom => "UTF-8 com BOM",
            Encoding::Utf16Le => "UTF-16 LE",
            Encoding::Utf16Be => "UTF-16 BE",
            Encoding::Windows1252 => "ANSI (Windows-1252)",
        };
        f.write_str(s)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    CrLf,
    Cr,
}

impl LineEnding {
    pub const ALL: [LineEnding; 3] = [LineEnding::CrLf, LineEnding::Lf, LineEnding::Cr];

    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "\n",
            LineEnding::CrLf => "\r\n",
            LineEnding::Cr => "\r",
        }
    }
}

impl fmt::Display for LineEnding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LineEnding::Lf => "Unix (LF)",
            LineEnding::CrLf => "Windows (CRLF)",
            LineEnding::Cr => "Mac clássico (CR)",
        };
        f.write_str(s)
    }
}

/// Decodifica bytes detectando BOM; sem BOM tenta UTF-8 e cai para Windows-1252.
pub fn decode(bytes: &[u8]) -> (String, Encoding) {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]) {
        return (
            String::from_utf8_lossy(rest).into_owned(),
            Encoding::Utf8Bom,
        );
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE][..]) {
        let (s, _) = encoding_rs::UTF_16LE.decode_without_bom_handling(rest);
        return (s.into_owned(), Encoding::Utf16Le);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF][..]) {
        let (s, _) = encoding_rs::UTF_16BE.decode_without_bom_handling(rest);
        return (s.into_owned(), Encoding::Utf16Be);
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => (s.to_owned(), Encoding::Utf8),
        Err(_) => {
            let (s, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes);
            (s.into_owned(), Encoding::Windows1252)
        }
    }
}

/// Codifica o texto no encoding pedido (incluindo BOM quando aplicável).
pub fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf8Bom => {
            let mut out = vec![0xEF, 0xBB, 0xBF];
            out.extend_from_slice(text.as_bytes());
            out
        }
        Encoding::Utf16Le => {
            let mut out = vec![0xFF, 0xFE];
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_le_bytes());
            }
            out
        }
        Encoding::Utf16Be => {
            let mut out = vec![0xFE, 0xFF];
            for unit in text.encode_utf16() {
                out.extend_from_slice(&unit.to_be_bytes());
            }
            out
        }
        Encoding::Windows1252 => {
            let (bytes, _, _) = encoding_rs::WINDOWS_1252.encode(text);
            bytes.into_owned()
        }
    }
}

/// Detecta o fim de linha predominante; `None` se o texto não tem quebras.
pub fn detect_eol(text: &str) -> Option<LineEnding> {
    let (mut lf, mut crlf, mut cr) = (0usize, 0usize, 0usize);
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                crlf += 1;
                i += 1;
            }
            b'\r' => cr += 1,
            b'\n' => lf += 1,
            _ => {}
        }
        i += 1;
    }
    if lf + crlf + cr == 0 {
        None
    } else if crlf >= lf && crlf >= cr {
        Some(LineEnding::CrLf)
    } else if lf >= cr {
        Some(LineEnding::Lf)
    } else {
        Some(LineEnding::Cr)
    }
}

/// Converte qualquer fim de linha para `\n` (formato interno do buffer).
pub fn normalize_eol(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Converte `\n` internos para o fim de linha de destino.
pub fn apply_eol(text: &str, eol: LineEnding) -> String {
    match eol {
        LineEnding::Lf => text.to_owned(),
        _ => text.replace('\n', eol.as_str()),
    }
}

pub struct Document {
    /// Texto sempre com `\n` como quebra de linha.
    pub text: String,
    pub path: Option<PathBuf>,
    pub encoding: Encoding,
    pub eol: LineEnding,
    pub modified: bool,
}

impl Document {
    pub fn new(eol: LineEnding) -> Self {
        Self {
            text: String::new(),
            path: None,
            encoding: Encoding::Utf8,
            eol,
            modified: false,
        }
    }

    pub fn from_bytes(bytes: &[u8], default_eol: LineEnding) -> Self {
        let (raw, encoding) = decode(bytes);
        let eol = detect_eol(&raw).unwrap_or(default_eol);
        Self {
            text: normalize_eol(&raw),
            path: None,
            encoding,
            eol,
            modified: false,
        }
    }

    pub fn open(path: &Path, default_eol: LineEnding) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("Não foi possível ler {}", path.display()))?;
        let mut doc = Self::from_bytes(&bytes, default_eol);
        doc.path = Some(path.to_path_buf());
        Ok(doc)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        encode(&apply_eol(&self.text, self.eol), self.encoding)
    }

    pub fn save_as(&mut self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_bytes())
            .with_context(|| format!("Não foi possível salvar {}", path.display()))?;
        self.path = Some(path.to_path_buf());
        self.modified = false;
        Ok(())
    }

    pub fn file_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sem título".to_owned())
    }

    /// Converte índice de caractere em (linha, coluna), ambos a partir de 1.
    pub fn line_col(&self, char_index: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for c in self.text.chars().take(char_index) {
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// Índice de caractere do início da linha `line` (a partir de 1).
    pub fn line_start(&self, line: usize) -> Option<usize> {
        if line == 0 {
            return None;
        }
        if line == 1 {
            return Some(0);
        }
        let mut current = 1;
        for (i, c) in self.text.chars().enumerate() {
            if c == '\n' {
                current += 1;
                if current == line {
                    return Some(i + 1);
                }
            }
        }
        None
    }

    pub fn line_count(&self) -> usize {
        self.text.chars().filter(|&c| c == '\n').count() + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_utf8() {
        let (s, e) = decode("olá".as_bytes());
        assert_eq!(s, "olá");
        assert_eq!(e, Encoding::Utf8);
    }

    #[test]
    fn detects_utf8_bom() {
        let (s, e) = decode(&[0xEF, 0xBB, 0xBF, b'a']);
        assert_eq!(s, "a");
        assert_eq!(e, Encoding::Utf8Bom);
    }

    #[test]
    fn detects_utf16() {
        let le = encode("ação", Encoding::Utf16Le);
        assert_eq!(decode(&le), ("ação".to_owned(), Encoding::Utf16Le));
        let be = encode("ação", Encoding::Utf16Be);
        assert_eq!(decode(&be), ("ação".to_owned(), Encoding::Utf16Be));
    }

    #[test]
    fn falls_back_to_windows_1252() {
        // "olá" em Windows-1252: á = 0xE1 (UTF-8 inválido)
        let (s, e) = decode(&[b'o', b'l', 0xE1]);
        assert_eq!(s, "olá");
        assert_eq!(e, Encoding::Windows1252);
    }

    #[test]
    fn roundtrip_all_encodings() {
        for enc in Encoding::ALL {
            let bytes = encode("Olá, ç!", enc);
            assert_eq!(decode(&bytes), ("Olá, ç!".to_owned(), enc), "{enc}");
        }
    }

    #[test]
    fn eol_detection() {
        assert_eq!(detect_eol("a\r\nb\r\n"), Some(LineEnding::CrLf));
        assert_eq!(detect_eol("a\nb\n"), Some(LineEnding::Lf));
        assert_eq!(detect_eol("a\rb\r"), Some(LineEnding::Cr));
        assert_eq!(detect_eol("abc"), None);
    }

    #[test]
    fn eol_preserved_on_save() {
        let original = b"linha 1\r\nlinha 2\r\n";
        let doc = Document::from_bytes(original, LineEnding::Lf);
        assert_eq!(doc.text, "linha 1\nlinha 2\n");
        assert_eq!(doc.eol, LineEnding::CrLf);
        assert_eq!(doc.to_bytes(), original);
    }

    #[test]
    fn encoding_and_bom_preserved_on_save() {
        let original = encode("x\ny", Encoding::Utf16Le);
        let doc = Document::from_bytes(&original, LineEnding::CrLf);
        assert_eq!(doc.encoding, Encoding::Utf16Le);
        assert_eq!(doc.eol, LineEnding::Lf);
        assert_eq!(doc.to_bytes(), original);
    }

    #[test]
    fn new_document_uses_default_eol() {
        let doc = Document::from_bytes(b"sem quebra", LineEnding::CrLf);
        assert_eq!(doc.eol, LineEnding::CrLf);
    }

    #[test]
    fn line_col_and_line_start() {
        let doc = Document::from_bytes("ab\ncdé\nf".as_bytes(), LineEnding::Lf);
        assert_eq!(doc.line_col(0), (1, 1));
        assert_eq!(doc.line_col(4), (2, 2));
        assert_eq!(doc.line_start(2), Some(3));
        assert_eq!(doc.line_start(3), Some(7));
        assert_eq!(doc.line_start(4), None);
        assert_eq!(doc.line_count(), 3);
    }

    #[test]
    fn save_and_reopen() -> Result<()> {
        let dir = std::env::temp_dir().join(format!("notepad-rs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("t.txt");
        let mut doc = Document::new(LineEnding::CrLf);
        doc.text = "a\nb".to_owned();
        doc.encoding = Encoding::Utf8Bom;
        doc.modified = true;
        doc.save_as(&path)?;
        assert!(!doc.modified);
        assert_eq!(std::fs::read(&path)?, b"\xEF\xBB\xBFa\r\nb");
        let reopened = Document::open(&path, LineEnding::Lf)?;
        assert_eq!(reopened.text, "a\nb");
        assert_eq!(reopened.eol, LineEnding::CrLf);
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }
}
