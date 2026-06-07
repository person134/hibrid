use std::process::Command;
use crate::backend::PackageManager;

pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub size: String,
    pub repository: String,
}

fn parse_search_output(output: &str, pkg_manager: &str) -> (String, String) {
    match pkg_manager {
        "pacman" | "yay" | "paru" => {
            let mut repo = String::new();
            let mut size = String::new();
            for line in output.lines() {
                if line.starts_with("Repository") {
                    repo = line.split(':').nth(1).unwrap_or("").trim().to_string();
                }
                if line.starts_with("Installed Size") {
                    size = line.split(':').nth(1).unwrap_or("").trim().to_string();
                }
            }
            if repo.is_empty() {
                (String::new(), String::new())
            } else {
                (repo, if size.is_empty() { "available".to_string() } else { size })
            }
        }
        "apt" => {
            for line in output.lines() {
                if line.starts_with("Size:") {
                    let size = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    return (String::new(), size);
                }
            }
            (String::new(), String::new())
        }
        "dnf" => {
            for line in output.lines() {
                if line.contains("Size") {
                    let size = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    return (String::new(), size);
                }
            }
            (String::new(), String::new())
        }
        "emerge" => {
            for line in output.lines() {
                if line.trim_start().starts_with("*") {
                    return ("portage".to_string(), "available".to_string());
                }
            }
            (String::new(), String::new())
        }
        "brew" => {
            for line in output.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("==") || trimmed.is_empty() {
                    continue;
                }
                return ("homebrew".to_string(), "available".to_string());
            }
            (String::new(), String::new())
        }
        _ => (String::new(), String::new()),
    }
}

fn get_package_info(program: &str, args: &[&str], pkg_manager: &str) -> (String, String) {
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            parse_search_output(&stdout, pkg_manager)
        }
        Err(_) => (String::new(), String::new()),
    }
}

pub fn search_info(manager: &PackageManager, package: &str) -> (String, String) {
    let mut args = manager.search_args.to_vec();
    args.push(package);
    get_package_info(manager.program, &args, manager.program)
}

fn parse_installed_output(output: &str, pkg_manager: &str) -> (String, String) {
    match pkg_manager {
        "pacman" | "yay" | "paru" => {
            let mut size = String::new();
            for line in output.lines() {
                if line.starts_with("Installed Size") {
                    size = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    break;
                }
            }
            (String::new(), size)
        }
        "apt" => {
            for line in output.lines() {
                if line.starts_with("Size:") {
                    let size = line.split(':').nth(1).unwrap_or("").trim().to_string();
                    return (String::new(), size);
                }
            }
            (String::new(), String::new())
        }
        "dnf" => {
            for line in output.lines() {
                if line.contains("installed") {
                    let size = line.trim().to_string();
                    return (String::new(), size);
                }
            }
            (String::new(), String::new())
        }
        "emerge" => {
            if !output.trim().is_empty() {
                return (String::new(), "installed".to_string());
            }
            (String::new(), String::new())
        }
        "brew" => {
            for line in output.lines() {
                if line.contains("files,") {
                    if let Some(size_part) = line.split(',').nth(1) {
                        return (String::new(), size_part.trim().to_string());
                    }
                }
            }
            if !output.trim().is_empty() {
                return (String::new(), "installed".to_string());
            }
            (String::new(), String::new())
        }
        _ => (String::new(), String::new()),
    }
}

pub fn get_installed_package_info(manager: &PackageManager, package: &str) -> (String, String) {
    let args: Vec<&str> = match manager.program {
        "pacman" | "yay" | "paru" => vec!["-Qi", package],
        "apt" => vec!["show", package],
        "dnf" => vec!["list", "installed", package],
        "emerge" => vec!["--info", package],
        "brew" => vec!["info", "--installed", package],
        _ => return (String::new(), String::new()),
    };

    match Command::new(manager.program).args(&args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if output.status.success() {
                parse_installed_output(&stdout, manager.program)
            } else {
                (String::new(), String::new())
            }
        }
        Err(_) => (String::new(), String::new()),
    }
}

pub fn is_flatpak_installed(app_id: &str) -> bool {
    match Command::new("flatpak").args(&["list", "--app", "--columns", "application"]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| line.trim() == app_id)
        }
        Err(_) => false,
    }
}

pub fn fuzzy_match_flatpak(query: &str) -> Option<String> {
    fuzzy_match_flatpak_with_size(query).map(|(id, _)| id)
}

pub fn fuzzy_match_flatpak_with_size(query: &str) -> Option<(String, String)> {
    match Command::new("flatpak").args(&["search", query]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("Name") || line.trim().is_empty() {
                    continue;
                }

                let mut app_id = String::new();
                let parts: Vec<&str> = line.split_whitespace().collect();

                for part in parts.iter() {
                    if (part.starts_with("org.") || part.starts_with("com.")) && part.contains('.') {
                        app_id = part.to_string();
                        break;
                    }
                }

                if !app_id.is_empty() {
                    let size = get_flatpak_size(&app_id);
                    return Some((app_id, size));
                }
            }
            None
        }
        Err(_) => None,
    }
}

fn get_flatpak_size(app_id: &str) -> String {
    match Command::new("flatpak").args(&["remote-info", "flathub", app_id]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("Download:") {
                    if let Some(size) = line.split(':').nth(1) {
                        return size.trim().to_string();
                    }
                }
            }
            String::new()
        }
        Err(_) => String::new(),
    }
}

pub fn search_package_linux(package: &str, manager: &PackageManager) -> Option<SearchResult> {
    let (repo, size) = search_info(manager, package);

    if size.is_empty() {
        return None;
    }

    let version = extract_version_from_manager(package, manager);
    let description = extract_description_from_manager(package, manager);

    Some(SearchResult {
        name: package.to_string(),
        version,
        description,
        size,
        repository: repo,
    })
}

fn extract_version_from_manager(package: &str, manager: &PackageManager) -> String {
    match manager.program {
        "pacman" | "yay" | "paru" => {
            let output = match Command::new(manager.program).args(&["-Si", package]).output() {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Version") {
                    if let Some(version) = line.split(':').nth(1) {
                        return version.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "apt" => {
            let output = match Command::new(manager.program)
                .args(&["show", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Version:") {
                    if let Some(v) = line.split(':').nth(1) {
                        return v.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "dnf" => {
            let output = match Command::new(manager.program).args(&["info", package]).output() {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Version") {
                    if let Some(v) = line.split(':').nth(1) {
                        return v.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "emerge" => {
            let output = match Command::new(manager.program)
                .args(&["--search", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("Latest version available") {
                    if let Some(v) = line.split(':').nth(1) {
                        return v.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "brew" => {
            let output = match Command::new(manager.program)
                .args(&["info", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("stable") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    for (i, part) in parts.iter().enumerate() {
                        if *part == "stable" {
                            if let Some(v) = parts.get(i + 1) {
                                return v.trim_end_matches(',').to_string();
                            }
                        }
                    }
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

fn extract_description_from_manager(package: &str, manager: &PackageManager) -> String {
    match manager.program {
        "pacman" | "yay" | "paru" => {
            let output = match Command::new(manager.program)
                .args(&["-Si", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Description") {
                    if let Some(desc) = line.split(':').nth(1) {
                        return desc.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "apt" => {
            let output = match Command::new(manager.program)
                .args(&["show", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Description:") {
                    if let Some(d) = line.splitn(2, ':').nth(1) {
                        return d.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "dnf" => {
            let output = match Command::new(manager.program)
                .args(&["info", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.starts_with("Summary") {
                    if let Some(d) = line.split(':').nth(1) {
                        return d.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "emerge" => {
            let output = match Command::new(manager.program)
                .args(&["--search", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.trim_start().starts_with("Description") {
                    if let Some(d) = line.split(':').nth(1) {
                        return d.trim().to_string();
                    }
                }
            }
            String::new()
        }
        "brew" => {
            let output = match Command::new(manager.program)
                .args(&["info", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() > 1 {
                let desc = lines[1].trim();
                if !desc.is_empty() && !desc.starts_with("==>") && !desc.starts_with('/') {
                    return desc.to_string();
                }
            }
            String::new()
        }
        _ => String::new(),
    }
}

pub fn search_package_flatpak(package: &str) -> Option<SearchResult> {
    let (app_id, size) = fuzzy_match_flatpak_with_size(package)?;

    let version = extract_flatpak_version(&app_id);
    let description = extract_flatpak_description(&app_id);

    Some(SearchResult {
        name: package.to_string(),
        version,
        description,
        size,
        repository: "flathub".to_string(),
    })
}

fn extract_flatpak_version(app_id: &str) -> String {
    match Command::new("flatpak")
        .args(&["remote-info", "flathub", app_id])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("Version:") {
                    if let Some(version) = line.split(':').nth(1) {
                        return version.trim().to_string();
                    }
                }
            }
            String::new()
        }
        Err(_) => String::new(),
    }
}

fn extract_flatpak_description(app_id: &str) -> String {
    match Command::new("flatpak")
        .args(&["remote-info", "flathub", app_id])
        .output()
    {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);

            if let Some(first_line) = stdout.lines().next() {
                let trimmed = first_line.trim();
                if let Some(pos) = trimmed.find(" - ") {
                    return trimmed[pos + 3..].trim().to_string();
                }
            }
            String::new()
        }
        Err(_) => String::new(),
    }
}
