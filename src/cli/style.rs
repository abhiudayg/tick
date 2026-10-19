use std::io::IsTerminal;

fn use_color() -> bool {
    std::io::stdout().is_terminal() && std::env::var("NO_COLOR").is_err()
}

pub fn bold(s: &str) -> String {
    if use_color() {
        format!("\x1b[1m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn dim(s: &str) -> String {
    if use_color() {
        format!("\x1b[2m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn green(s: &str) -> String {
    if use_color() {
        format!("\x1b[32m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn yellow(s: &str) -> String {
    if use_color() {
        format!("\x1b[33m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn blue(s: &str) -> String {
    if use_color() {
        format!("\x1b[34m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn magenta(s: &str) -> String {
    if use_color() {
        format!("\x1b[35m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn cyan(s: &str) -> String {
    if use_color() {
        format!("\x1b[36m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn red(s: &str) -> String {
    if use_color() {
        format!("\x1b[31m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn status_color(status: &str) -> String {
    let lower = status.to_lowercase();
    if lower.contains("done") || lower.contains("closed") || lower.contains("resolved") {
        green(status)
    } else if lower.contains("progress") || lower.contains("review") || lower.contains("testing") {
        yellow(status)
    } else if lower.contains("block") || lower.contains("cancel") || lower.contains("wont") {
        red(status)
    } else {
        blue(status)
    }
}

pub fn priority_color(priority: &str) -> String {
    match priority.to_lowercase().as_str() {
        "highest" | "blocker" | "p1" => red(priority),
        "high" | "critical" | "p2" => yellow(priority),
        "medium" | "major" | "p3" => cyan(priority),
        "low" | "minor" | "p4" => dim(priority),
        "lowest" | "trivial" | "p5" => dim(priority),
        _ => priority.to_string(),
    }
}
