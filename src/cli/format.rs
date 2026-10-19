use super::style;

/// Print a clean aligned terminal table (like gh's table printer)
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if headers.is_empty() {
        return;
    }
    let col_count = headers.len();
    let mut widths = vec![0usize; col_count];

    for (i, h) in headers.iter().enumerate() {
        widths[i] = h.len();
    }

    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                // Strip ANSI escape codes when computing length
                let visible_len = visible_length(cell);
                if visible_len > widths[i] {
                    widths[i] = visible_len;
                }
            }
        }
    }

    // Print headers in bold
    let header_line = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let pad = " ".repeat(widths[i].saturating_sub(h.len()));
            format!("{}{pad}", style::bold(h))
        })
        .collect::<Vec<_>>()
        .join("   ");
    println!("{header_line}");

    // Print rows
    for row in rows {
        let line = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let vlen = visible_length(cell);
                let col_width = widths.get(i).copied().unwrap_or(vlen);
                let pad = " ".repeat(col_width.saturating_sub(vlen));
                format!("{cell}{pad}")
            })
            .collect::<Vec<_>>()
            .join("   ");
        println!("{line}");
    }
}

pub fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else if max_chars > 3 {
        let sub: String = chars[..max_chars - 3].iter().collect();
        format!("{sub}...")
    } else {
        chars[..max_chars].iter().collect()
    }
}

fn visible_length(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_length_ignores_ansi() {
        assert_eq!(visible_length("hello"), 5);
        assert_eq!(visible_length("\x1b[32mhello\x1b[0m"), 5);
        assert_eq!(visible_length("\x1b[1;31mfoo\x1b[0m bar"), 7);
    }

    #[test]
    fn truncate_adds_ellipsis() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("longer text to truncate", 10), "longer ...");
    }
}
