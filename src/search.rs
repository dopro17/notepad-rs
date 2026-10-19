//! Localizar e substituir, trabalhando com índices de caractere (não bytes).

#[derive(Clone, Copy, Debug)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub wrap_around: bool,
}

fn chars_eq(a: char, b: char, case_sensitive: bool) -> bool {
    if case_sensitive {
        a == b
    } else {
        a == b || a.to_lowercase().eq(b.to_lowercase())
    }
}

fn matches_at(hay: &[char], needle: &[char], at: usize, case_sensitive: bool) -> bool {
    at + needle.len() <= hay.len()
        && needle
            .iter()
            .zip(&hay[at..])
            .all(|(&n, &h)| chars_eq(n, h, case_sensitive))
}

/// Procura `query` a partir de `from` (índice de caractere).
/// Para frente: primeira ocorrência começando em `from` ou depois.
/// Para trás: última ocorrência que termina em `from` ou antes.
/// Retorna o intervalo `[início, fim)` em caracteres.
pub fn find(
    text: &str,
    query: &str,
    from: usize,
    forward: bool,
    opts: SearchOptions,
) -> Option<(usize, usize)> {
    if query.is_empty() {
        return None;
    }
    let hay: Vec<char> = text.chars().collect();
    let needle: Vec<char> = query.chars().collect();
    if needle.len() > hay.len() {
        return None;
    }
    let last = hay.len() - needle.len();
    let from = from.min(hay.len());
    let hit = |i: usize| matches_at(&hay, &needle, i, opts.case_sensitive);

    let found = if forward {
        (from..=last)
            .find(|&i| hit(i))
            .or_else(|| opts.wrap_around.then(|| (0..=last).find(|&i| hit(i)))?)
    } else {
        let end = from.checked_sub(needle.len()).map(|e| e.min(last));
        end.and_then(|e| (0..=e).rev().find(|&i| hit(i)))
            .or_else(|| {
                opts.wrap_around
                    .then(|| (0..=last).rev().find(|&i| hit(i)))?
            })
    };
    found.map(|i| (i, i + needle.len()))
}

/// Substitui todas as ocorrências. Retorna o novo texto e a quantidade.
pub fn replace_all(
    text: &str,
    query: &str,
    replacement: &str,
    case_sensitive: bool,
) -> (String, usize) {
    if query.is_empty() {
        return (text.to_owned(), 0);
    }
    let hay: Vec<char> = text.chars().collect();
    let needle: Vec<char> = query.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut count = 0;
    let mut i = 0;
    while i < hay.len() {
        if matches_at(&hay, &needle, i, case_sensitive) {
            out.push_str(replacement);
            i += needle.len();
            count += 1;
        } else {
            out.push(hay[i]);
            i += 1;
        }
    }
    (out, count)
}

/// Substitui o intervalo de caracteres `[start, end)` por `replacement`.
pub fn replace_range(text: &mut String, start: usize, end: usize, replacement: &str) {
    let byte = |ci: usize| text.char_indices().nth(ci).map_or(text.len(), |(b, _)| b);
    let (bs, be) = (byte(start), byte(end));
    text.replace_range(bs..be, replacement);
}

/// Verifica se o intervalo `[start, end)` contém exatamente `query`.
pub fn range_matches(
    text: &str,
    start: usize,
    end: usize,
    query: &str,
    case_sensitive: bool,
) -> bool {
    let selected: Vec<char> = text
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();
    let needle: Vec<char> = query.chars().collect();
    !needle.is_empty()
        && selected.len() == needle.len()
        && matches_at(&selected, &needle, 0, case_sensitive)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WRAP: SearchOptions = SearchOptions {
        case_sensitive: false,
        wrap_around: true,
    };
    const NOWRAP_CASE: SearchOptions = SearchOptions {
        case_sensitive: true,
        wrap_around: false,
    };

    #[test]
    fn find_forward_and_wrap() {
        let t = "abc ABC abc";
        assert_eq!(find(t, "abc", 1, true, WRAP), Some((4, 7)));
        assert_eq!(find(t, "abc", 9, true, WRAP), Some((0, 3)));
        assert_eq!(find(t, "abc", 1, true, NOWRAP_CASE), Some((8, 11)));
        assert_eq!(find(t, "abc", 9, true, NOWRAP_CASE), None);
    }

    #[test]
    fn find_backward() {
        let t = "abc abc abc";
        assert_eq!(find(t, "abc", 8, false, WRAP), Some((4, 7)));
        assert_eq!(find(t, "abc", 2, false, WRAP), Some((8, 11)));
        assert_eq!(find(t, "abc", 2, false, NOWRAP_CASE), None);
    }

    #[test]
    fn find_uses_char_indices() {
        let t = "ção ção";
        assert_eq!(find(t, "ÇÃO", 1, true, WRAP), Some((4, 7)));
    }

    #[test]
    fn replace_all_counts() {
        assert_eq!(
            replace_all("a.A.a", "a", "xy", false),
            ("xy.xy.xy".to_owned(), 3)
        );
        assert_eq!(
            replace_all("a.A.a", "a", "xy", true),
            ("xy.A.xy".to_owned(), 2)
        );
        assert_eq!(replace_all("abc", "", "x", true), ("abc".to_owned(), 0));
    }

    #[test]
    fn replace_range_chars() {
        let mut s = "olá mundo".to_owned();
        replace_range(&mut s, 2, 3, "a");
        assert_eq!(s, "ola mundo");
        assert!(range_matches("Olá", 0, 3, "olá", false));
        assert!(!range_matches("Olá", 0, 3, "olá", true));
    }
}
