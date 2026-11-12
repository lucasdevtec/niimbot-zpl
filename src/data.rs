//! Variable data: CSV import and `{{name}}` substitution.

/// Parses CSV/TSV text (first line = header). Delimiter is auto-detected (`;`, tab or `,`).
pub fn parse_csv(text: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let text = text.trim_start_matches('\u{feff}');
    let first = text.lines().next().unwrap_or("");
    let delim = [';', '\t', ',']
        .into_iter()
        .find(|d| first.contains(*d))
        .unwrap_or(',');
    let mut lines = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| split_line(l, delim));
    let headers: Vec<String> = lines
        .next()
        .unwrap_or_default()
        .into_iter()
        .map(|h| h.trim().to_string())
        .collect();
    let rows = lines.collect();
    (headers, rows)
}

fn split_line(line: &str, delim: char) -> Vec<String> {
    let (mut out, mut cur, mut q) = (vec![], String::new(), false);
    let mut it = line.trim_end_matches('\r').chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if q && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => q = !q,
            c if c == delim && !q => out.push(std::mem::take(&mut cur)),
            c => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Replaces `{{header}}` with the row's value (ZPL control characters are stripped from values).
pub fn substitute(template: &str, headers: &[String], row: Option<&Vec<String>>) -> String {
    let mut out = template.to_string();
    for (i, h) in headers.iter().enumerate() {
        let v = row
            .and_then(|r| r.get(i))
            .map(|v| v.replace(['^', '~'], ""))
            .unwrap_or_default();
        out = out.replace(&format!("{{{{{h}}}}}"), &v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_and_subst() {
        let (h, r) = parse_csv("nome;setor\nAna;TI\n\"Silva; Jo\";RH\n");
        assert_eq!(h, ["nome", "setor"]);
        assert_eq!(r.len(), 2);
        assert_eq!(
            substitute("{{nome}}/{{setor}}", &h, Some(&r[1])),
            "Silva; Jo/RH"
        );
    }
}
