use colored::*;
use std::io::{self, Write};
use crate::action::Action;

#[allow(dead_code)]
pub fn format_box(title: &str, package: &str, repo: &str, size: &str) -> String {
    let width = 40;
    let mut title_str = format!(" {} ", title);
    if title_str.len() > width - 4 {
        title_str = format!(" {} ", &title[..title.len().saturating_sub(title_str.len() - (width - 4))]);
    }

    let mut result = String::new();
    let dashes = "─".repeat(width - title_str.len() - 2);
    result.push_str(&format!("┌{}{}┐\n", title_str, dashes));

    let pkg_line = format!("Package: {}", package);
    result.push_str(&format!("│ {:<36} │\n", pkg_line));

    if !repo.is_empty() {
        let repo_line = format!("Repository: {}", repo);
        result.push_str(&format!("│ {:<36} │\n", repo_line));
    }

    if !size.is_empty() {
        let size_line = format!("Size: {}", size);
        result.push_str(&format!("│ {:<36} │\n", size_line));
    }

    result.push_str(&format!("└{}┘\n", "─".repeat(width - 2)));
    result
}

pub fn format_box_multiple(title: &str, packages_info: Vec<(String, String, String)>) -> String {
    let width = 40;
    let mut title_str = format!(" {} ", title);
    if title_str.len() > width - 4 {
        title_str = format!(" {} ", &title[..title.len().saturating_sub(title_str.len() - (width - 4))]);
    }

    let mut result = String::new();
    let dashes = "─".repeat(width - title_str.len() - 2);
    result.push_str(&format!("┌{}{}┐\n", title_str, dashes));

    for (pkg, repo, size) in packages_info {
        let pkg_line = if !repo.is_empty() && !size.is_empty() {
            format!("{} ({}) - {}", pkg, repo, size)
        } else if !size.is_empty() {
            format!("{} - {}", pkg, size)
        } else if !repo.is_empty() {
            format!("{} ({})", pkg, repo)
        } else {
            pkg
        };
        result.push_str(&format!("│ {:<36} │\n", pkg_line));
    }

    result.push_str(&format!("└{}┘\n", "─".repeat(width - 2)));
    result
}

pub fn format_search_box(package: &str, result: &crate::search::SearchResult) -> String {
    let width = 45;
    let title = format!("Search: {}", package);
    let mut title_str = format!(" {} ", title);
    if title_str.len() > width - 4 {
        title_str = format!(" {} ", &title[..title.len().saturating_sub(title_str.len() - (width - 4))]);
    }

    let mut box_str = String::new();
    let inner_width = width - 4;

    let dashes = "─".repeat(width.saturating_sub(title_str.len() + 2));
    box_str.push_str(&format!("┌{}{}┐\n", title_str, dashes));

    let pkg_line = format!("Package: {}", result.name);
    box_str.push_str(&format!("│ {:<width$} │\n", pkg_line, width = inner_width));

    if !result.version.is_empty() {
        let ver_line = format!("Version: {}", result.version);
        box_str.push_str(&format!("│ {:<width$} │\n", ver_line, width = inner_width));
    }

    if !result.repository.is_empty() {
        let repo_line = format!("Repository: {}", result.repository);
        box_str.push_str(&format!("│ {:<width$} │\n", repo_line, width = inner_width));
    }

    if !result.size.is_empty() {
        let size_line = format!("Size: {}", result.size);
        box_str.push_str(&format!("│ {:<width$} │\n", size_line, width = inner_width));
    }

    if !result.description.is_empty() {
        let desc = if result.description.len() > inner_width - 14 {
            format!("{}...", &result.description[..inner_width - 17])
        } else {
            result.description.clone()
        };
        let desc_line = format!("Description: {}", desc);
        box_str.push_str(&format!("│ {:<width$} │\n", desc_line, width = inner_width));
    }

    box_str.push_str(&format!("└{}┘\n", "─".repeat(width - 2)));
    box_str
}

pub fn print_result(action: Action, success: bool) {
    let action_name = match action {
        Action::Install => "Install",
        Action::Remove => "Removal",
        Action::Update => "Update",
        _ => return,
    };
    let result = if success { "finished" } else { "failed" };
    let colored = format!("{} {}", action_name, result);
    if success {
        println!("{}", colored.green());
    } else {
        println!("{}", colored.red());
    }
}

pub fn ask_confirmation() -> bool {
    print!("{} {} {} ", "?".bright_cyan().bold(), "Proceed with".bright_white(), "installation?".green().bold());
    print!("{} ", "(Y/n):".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes")
}

pub fn ask_removal_confirmation() -> bool {
    print!("{} {} {} ", "!".bright_red().bold(), "Remove this".bright_white(), "package?".red().bold());
    print!("{} ", "(Y/n):".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes")
}

pub fn ask_flatpak_install() -> bool {
    println!("{}", "Flatpak is required but not installed.".yellow());
    print!("{} {} {} ", "?".bright_cyan().bold(), "Install Flatpak now?".bright_white(), "(Y/n):".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes")
}

pub fn ask_update_confirmation() -> bool {
    print!("{} {} {} ", "⟳".bright_yellow().bold(), "Update this".bright_white(), "package?".yellow().bold());
    print!("{} ", "(Y/n):".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let trimmed = input.trim();
    trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y") || trimmed.eq_ignore_ascii_case("yes")
}
