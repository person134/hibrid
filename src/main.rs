use std::env;
use std::process::{Command, Stdio};
use std::process::exit;
use std::io::{self, Write};
use colored::*;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

/// Supported operating systems
#[derive(Debug, PartialEq)]
enum System {
    Windows,
    Linux,
    MacOS,
    Unknown,
}

/// Supported actions
#[derive(Debug, Clone, Copy)]
enum Action {
    Install,
    InstallQuiet,
    InstallAutoinstall,
    InstallAutoinstallQuiet,
    InstallFlatpak,
    InstallFlatpakQuiet,
    InstallAutoinstallFlatpak,
    InstallAutoinstallFlatpakQuiet,
    Remove,
    RemoveQuiet,
    RemoveAutoinstall,
    RemoveAutoinstallQuiet,
    RemoveFlatpak,
    RemoveFlatpakQuiet,
    RemoveAutoinstallFlatpak,
    RemoveAutoinstallFlatpakQuiet,
    Update,
    UpdateQuiet,
    UpdateAutoinstall,
    UpdateAutoinstallQuiet,
    UpdateFlatpak,
    UpdateFlatpakQuiet,
    UpdateAutoinstallFlatpak,
    UpdateAutoinstallFlatpakQuiet,
    List,
    ListFlatpak,
    Search,
    SearchFlatpak,
}

/// Represents a package manager and its commands
struct PackageManager {
    program: &'static str,
    install_args: &'static [&'static str],
    remove_args: &'static [&'static str],
    update_args: &'static [&'static str],
    update_single_args: &'static [&'static str],
    list_args: &'static [&'static str],
    search_args: &'static [&'static str],
}

impl PackageManager {
    fn run(&self, action: Action, package: &str) -> (bool, String) {
        let mut args: Vec<&str> = match action {
            Action::Install => self.install_args.to_vec(),
            Action::Remove => self.remove_args.to_vec(),
            _ => return (false, String::new()),
        };

        args.push(package);
        run_command_with_output(self.program, &args, self.program)
    }

    fn search_info(&self, package: &str) -> (String, String) {
        let mut args = self.search_args.to_vec();
        args.push(package);
        get_package_info(self.program, &args, self.program)
    }
}

/// Runs a system command and captures output
fn run_command_with_output(program: &str, args: &[&str], pkg_manager: &str) -> (bool, String) {
    run_command_with_output_detailed(program, args, pkg_manager, false)
}

/// Shows an animated spinner in quiet mode
fn show_spinner(running: Arc<AtomicBool>) {
    let spinner_chars = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut index = 0;
    
    while running.load(Ordering::Relaxed) {
        print!("\r{} Processing...", spinner_chars[index]);
        io::stdout().flush().unwrap();
        index = (index + 1) % spinner_chars.len();
        thread::sleep(Duration::from_millis(80));
    }
    print!("\r");
    io::stdout().flush().unwrap();
}

/// Runs a system command with optional detailed output
fn run_command_with_output_detailed(program: &str, args: &[&str], pkg_manager: &str, detailed: bool) -> (bool, String) {
    if detailed {
        // For detailed mode, show all output
        if program == "sudo" {
            match Command::new(program)
                .args(args)
                .stdin(Stdio::inherit())
                .status() {
                Ok(status) => (status.success(), String::new()),
                Err(_) => (false, String::new()),
            }
        } else {
            match Command::new(program)
                .args(args)
                .status() {
                Ok(status) => (status.success(), String::new()),
                Err(_) => (false, String::new()),
            }
        }
    } else {
        // For normal mode (quiet), show spinner and capture output
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        
        let spinner_thread = thread::spawn(move || {
            show_spinner(running_clone);
        });
        
        let result = if program == "sudo" {
            // For sudo, inherit stdin for interaction, capture output for extraction
            match Command::new(program)
                .args(args)
                .stdin(Stdio::inherit())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output() {
                Ok(output) => {
                    let success = output.status.success();
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let info = extract_package_info(&stdout, pkg_manager);
                    (success, info)
                }
                Err(_) => (false, String::new()),
            }
        } else {
            // For other commands, capture output to extract info
            match Command::new(program)
                .args(args)
                .output() {
                Ok(output) => {
                    let success = output.status.success();
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let info = extract_package_info(&stdout, pkg_manager);
                    (success, info)
                }
                Err(_) => (false, String::new()),
            }
        };
        
        running.store(false, Ordering::Relaxed);
        let _ = spinner_thread.join();
        
        result
    }
}

/// Extract package info from package manager output
fn extract_package_info(output: &str, pkg_manager: &str) -> String {
    match pkg_manager {
        "pacman" => {
            for line in output.lines() {
                if line.contains("Total Installed Size:") {
                    return line.trim().to_string();
                }
            }
        }
        "apt" => {
            for line in output.lines() {
                if line.contains("newly installed") || line.contains("upgraded") {
                    return line.trim().to_string();
                }
            }
        }
        "dnf" => {
            for line in output.lines() {
                if line.contains("Installed") || line.contains("Size") {
                    return line.trim().to_string();
                }
            }
        }
        "emerge" => {
            for line in output.lines() {
                if line.contains("Total:") || line.contains("ebuild") {
                    return line.trim().to_string();
                }
            }
        }
        "brew" => {
            for line in output.lines() {
                if line.contains("==>") {
                    return line.trim().to_string();
                }
            }
        }
        _ => {}
    }
    String::new()
}

/// Check if a flatpak is installed
fn is_flatpak_installed(app_id: &str) -> bool {
    match Command::new("flatpak").args(&["list", "--app", "--columns", "application"]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.lines().any(|line| line.trim() == app_id)
        }
        Err(_) => false,
    }
}

/// Fuzzy match flatpak app ID
fn fuzzy_match_flatpak(query: &str) -> Option<String> {
    fuzzy_match_flatpak_with_size(query).map(|(id, _)| id)
}

/// Fuzzy match flatpak app ID and extract size
fn fuzzy_match_flatpak_with_size(query: &str) -> Option<(String, String)> {
    match Command::new("flatpak").args(&["search", query]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Skip header line and empty lines
                if line.contains("Name") || line.trim().is_empty() {
                    continue;
                }

                let mut app_id = String::new();

                // Extract app ID from the line
                let parts: Vec<&str> = line.split_whitespace().collect();

                for part in parts.iter() {
                    if (part.starts_with("org.") || part.starts_with("com.")) && part.contains('.') {
                        app_id = part.to_string();
                        break;
                    }
                }

                if !app_id.is_empty() {
                    // Get size from flatpak info command
                    let size = get_flatpak_size(&app_id);
                    return Some((app_id, size));
                }
            }
            None
        }
        Err(_) => None,
    }
}

/// Get download size for a flatpak package
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

/// Get package info from search output
fn get_package_info(program: &str, args: &[&str], pkg_manager: &str) -> (String, String) {
    match Command::new(program).args(args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            parse_search_output(&stdout, pkg_manager)
        }
        Err(_) => (String::new(), String::new()),
    }
}

/// Check if a package is installed and get its size
fn get_installed_package_info(program: &str, package: &str, pkg_manager: &str) -> (String, String) {
    let args = match pkg_manager {
        "pacman" => vec!["-Qi", package],
        "apt" => vec!["show", package],
        "dnf" => vec!["list", "installed", package],
        "emerge" => vec!["--info", package],
        "brew" => vec!["info", "--installed", package],
        _ => return (String::new(), String::new()),
    };

    match Command::new(program).args(&args).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            if output.status.success() {
                parse_installed_output(&stdout, pkg_manager)
            } else {
                (String::new(), String::new())
            }
        }
        Err(_) => (String::new(), String::new()),
    }
}

/// Parse installed package output to extract size
fn parse_installed_output(output: &str, pkg_manager: &str) -> (String, String) {
    match pkg_manager {
        "pacman" => {
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
            // emerge --info output doesn't give per-package size cleanly;
            // treat a non-empty output as confirmation the package is installed
            if !output.trim().is_empty() {
                return (String::new(), "installed".to_string());
            }
            (String::new(), String::new())
        }
        "brew" => {
            // `brew info --installed <pkg>` outputs lines like:
            //   <name>: stable <version> (bottled)
            //   <path>: <n> files, <size>
            for line in output.lines() {
                if line.contains("files,") {
                    if let Some(size_part) = line.split(',').nth(1) {
                        return (String::new(), size_part.trim().to_string());
                    }
                }
            }
            // If brew info succeeded the package is installed; size just wasn't parseable
            if !output.trim().is_empty() {
                return (String::new(), "installed".to_string());
            }
            (String::new(), String::new())
        }
        _ => (String::new(), String::new()),
    }
}

/// Parse search output to extract size and repository info (returns tuple: repo, size)
fn parse_search_output(output: &str, pkg_manager: &str) -> (String, String) {
    match pkg_manager {
        "pacman" => {
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
            (repo, size)
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
            // `emerge --search <pkg>` output; grab the first size-like line
            for line in output.lines() {
                if line.trim_start().starts_with("*") {
                    // package found — size is not always shown, return a placeholder so
                    // the caller knows the package exists
                    return ("portage".to_string(), "available".to_string());
                }
            }
            (String::new(), String::new())
        }
        "brew" => {
            // `brew info <pkg>` first line: "<name>: stable <version> (bottled)"
            // size line: "/<cellar-path>: <n> files, <size>"
            let mut repo = String::new();
            let mut size = String::new();
            for line in output.lines() {
                if line.contains("files,") {
                    if let Some(s) = line.split(',').nth(1) {
                        size = s.trim().to_string();
                    }
                }
                if line.contains("homebrew") || line.contains("Homebrew") {
                    repo = "homebrew".to_string();
                }
            }
            // If we found size info, or at least the output was non-empty, the package exists
            if size.is_empty() && !output.trim().is_empty() {
                size = "available".to_string();
            }
            if repo.is_empty() && !output.trim().is_empty() {
                repo = "homebrew".to_string();
            }
            (repo, size)
        }
        _ => (String::new(), String::new()),
    }
}

/// Ask user for confirmation
fn ask_confirmation() -> bool {
    print!("{} {} {} ", "?".bright_cyan().bold(), "Proceed with".bright_white(), "installation?".green().bold());
    print!("{} ", "(Y/N):".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes")
}

/// Ask user for removal confirmation
fn ask_removal_confirmation() -> bool {
    print!("{} {} {} ", "!".bright_red().bold(), "Remove this".bright_white(), "package?".red().bold());
    print!("{} ", "(Y/N):".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes")
}

/// Ask user for update confirmation
fn ask_update_confirmation() -> bool {
    print!("{} {} {} ", "⟳".bright_yellow().bold(), "Update this".bright_white(), "package?".yellow().bold());
    print!("{} ", "(Y/N):".bright_black());
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes")
}


/// Check if a command exists (silently)
fn command_exists(program: &str) -> bool {
    // `which` works on Linux and macOS; on Windows use `where`
    let checker = if cfg!(target_os = "windows") { "where" } else { "which" };
    match Command::new(checker)
        .arg(program)
        .output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Detect operating system
fn detect_system() -> System {
    if cfg!(target_os = "windows") {
        System::Windows
    } else if cfg!(target_os = "linux") {
        System::Linux
    } else if cfg!(target_os = "macos") {
        System::MacOS
    } else {
        System::Unknown
    }
}

/// Detect available Linux package manager
fn detect_linux_package_manager() -> Option<PackageManager> {
    let managers = vec![
        PackageManager {
            program: "apt",
            install_args: &["install", "-y"],
            remove_args: &["remove", "-y"],
            update_args: &["upgrade", "-y"],
            update_single_args: &["install", "--only-upgrade", "-y"],
            list_args: &["list", "--installed"],
            search_args: &["cache", "show"],
        },
        PackageManager {
            program: "pacman",
            install_args: &["-S", "--noconfirm"],
            remove_args: &["-R", "--noconfirm"],
            update_args: &["-Syu", "--noconfirm"],
            update_single_args: &["-S", "--noconfirm"],
            list_args: &["-Q"],
            search_args: &["-Si"],
        },
        PackageManager {
            program: "dnf",
            install_args: &["install", "-y"],
            remove_args: &["remove", "-y"],
            update_args: &["upgrade", "-y"],
            update_single_args: &["upgrade", "-y"],
            list_args: &["list", "installed"],
            search_args: &["info"],
        },
        // Portage: --usepkg --getbinpkg restricts to binary packages only (no source builds)
        PackageManager {
            program: "emerge",
            install_args: &["--ask=n", "--usepkg", "--getbinpkg"],
            remove_args: &["--ask=n", "--unmerge"],
            update_args: &["--ask=n", "--usepkg", "--getbinpkg", "--update", "--deep", "--newuse", "@world"],
            update_single_args: &["--ask=n", "--usepkg", "--getbinpkg", "--update"],
            list_args: &["--list-sets"],
            search_args: &["--search"],
        },
    ];

    for manager in managers {
        if command_exists(manager.program) {
            return Some(manager);
        }
    }

    None
}

/// Detect available macOS package manager (Homebrew)
fn detect_macos_package_manager() -> Option<PackageManager> {
    if command_exists("brew") {
        Some(PackageManager {
            program: "brew",
            install_args: &["install"],
            remove_args: &["uninstall"],
            update_args: &["upgrade"],
            update_single_args: &["upgrade"],
            list_args: &["list"],
            search_args: &["info"],
        })
    } else {
        None
    }
}

fn parse_action(flag: &str) -> Option<Action> {
    if !flag.starts_with('-') || flag.len() < 2 {
        return None;
    }

    let flag_chars = &flag[1..];
    let base = flag_chars.chars().next().unwrap();

    if base == 'S' {
        let modifiers = &flag_chars[1..];
        // Validate that modifiers only contain 'f'
        if !modifiers.chars().all(|c| c == 'f') {
            return None;
        }
        let has_f = modifiers.contains('f');
        return match has_f {
            true => Some(Action::SearchFlatpak),
            false => if modifiers.is_empty() { Some(Action::Search) } else { None },
        };
    }

    if base == 'L' {
        let modifiers = &flag_chars[1..];
        // Validate that modifiers only contain 'f'
        if !modifiers.chars().all(|c| c == 'f') {
            return None;
        }
        let has_f = modifiers.contains('f');
        return match has_f {
            true => Some(Action::ListFlatpak),
            false => if modifiers.is_empty() { Some(Action::List) } else { None },
        };
    }

    let modifiers = &flag_chars[1..];

    // Validate that modifiers only contain valid characters (a, q, f)
    if !modifiers.chars().all(|c| c == 'a' || c == 'q' || c == 'f') {
        return None;
    }

    let has_a = modifiers.contains('a');
    let has_q = modifiers.contains('q');
    let has_f = modifiers.contains('f');

    match (base, has_a, has_q, has_f) {
        ('I', false, false, false) => Some(Action::Install),
        ('I', false, true, false) => Some(Action::InstallQuiet),
        ('I', true, false, false) => Some(Action::InstallAutoinstall),
        ('I', true, true, false) => Some(Action::InstallAutoinstallQuiet),
        ('I', false, false, true) => Some(Action::InstallFlatpak),
        ('I', false, true, true) => Some(Action::InstallFlatpakQuiet),
        ('I', true, false, true) => Some(Action::InstallAutoinstallFlatpak),
        ('I', true, true, true) => Some(Action::InstallAutoinstallFlatpakQuiet),
        ('R', false, false, false) => Some(Action::Remove),
        ('R', false, true, false) => Some(Action::RemoveQuiet),
        ('R', true, false, false) => Some(Action::RemoveAutoinstall),
        ('R', true, true, false) => Some(Action::RemoveAutoinstallQuiet),
        ('R', false, false, true) => Some(Action::RemoveFlatpak),
        ('R', false, true, true) => Some(Action::RemoveFlatpakQuiet),
        ('R', true, false, true) => Some(Action::RemoveAutoinstallFlatpak),
        ('R', true, true, true) => Some(Action::RemoveAutoinstallFlatpakQuiet),
        ('U', false, false, false) => Some(Action::Update),
        ('U', false, true, false) => Some(Action::UpdateQuiet),
        ('U', true, false, false) => Some(Action::UpdateAutoinstall),
        ('U', true, true, false) => Some(Action::UpdateAutoinstallQuiet),
        ('U', false, false, true) => Some(Action::UpdateFlatpak),
        ('U', false, true, true) => Some(Action::UpdateFlatpakQuiet),
        ('U', true, false, true) => Some(Action::UpdateAutoinstallFlatpak),
        ('U', true, true, true) => Some(Action::UpdateAutoinstallFlatpakQuiet),
        _ => None,
    }
}

/// Check if an action is in autoinstall mode
fn is_autoinstall(action: Action) -> bool {
    matches!(
        action,
        Action::InstallAutoinstall
            | Action::InstallAutoinstallQuiet
            | Action::InstallAutoinstallFlatpak
            | Action::InstallAutoinstallFlatpakQuiet
            | Action::RemoveAutoinstall
            | Action::RemoveAutoinstallQuiet
            | Action::RemoveAutoinstallFlatpak
            | Action::RemoveAutoinstallFlatpakQuiet
            | Action::UpdateAutoinstall
            | Action::UpdateAutoinstallQuiet
            | Action::UpdateAutoinstallFlatpak
            | Action::UpdateAutoinstallFlatpakQuiet
    )
}

struct SearchResult {
    name: String,
    version: String,
    description: String,
    size: String,
    repository: String,
}

fn format_search_box(package: &str, result: &SearchResult) -> String {
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

fn search_package_linux(package: &str, manager: &PackageManager) -> Option<SearchResult> {
    let (repo, size) = manager.search_info(package);

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
        "pacman" => {
            let output = match Command::new(manager.program)
                .args(&["-Si", package])
                .output()
            {
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
                .args(&["cache", "show", package])
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
            let output = match Command::new(manager.program)
                .args(&["info", package])
                .output()
            {
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
            // emerge --search outputs lines like "[ Searching ... ]" then
            //   *  category/package
            //        Latest version available: x.y.z
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
            // `brew info <pkg>` first content line: "<name>: stable <version> (bottled)"
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
        "pacman" => {
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
                .args(&["cache", "show", package])
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
            // `brew info <pkg>` has a description line after the formula header
            let output = match Command::new(manager.program)
                .args(&["info", package])
                .output()
            {
                Ok(out) => out,
                Err(_) => return String::new(),
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            // Typically: line 0 = "name: stable version", line 1 = description
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

fn search_package_flatpak(package: &str) -> Option<SearchResult> {
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
    match Command::new("flatpak").args(&["remote-info", "flathub", app_id]).output() {
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
    match Command::new("flatpak").args(&["remote-info", "flathub", app_id]).output() {
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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("{}", "╔════════════════════════════════════════════════════════════╗".bright_cyan());
        println!("{}", "║              Hibrid Package Manager Wrapper               ║".bright_cyan());
        println!("{}", "╚════════════════════════════════════════════════════════════╝".bright_cyan());
        println!();
        println!("{}", "Usage: hibrid [-I|-R|-U|-L][a][q][f] [pkg]".bright_white().bold());
        println!();
        println!("  {} Install package", "-I".green().bold());
        println!("  {} Remove package", "-R".red().bold());
        println!("  {} Update system or package", "-U".yellow().bold());
        println!("  {} List installed packages", "-L".cyan().bold());
        println!();
        println!("{}", "Modifiers:".bright_white().bold());
        println!("  {} Autoinstall (skip confirmation)", "a".bright_yellow());
        println!("  {} Quiet output (suppress package manager output)", "q".bright_yellow());
        println!("  {} Use Flatpak (Linux only)", "f".bright_magenta());
        println!();
        println!("{}", "Supported backends:".bright_white().bold());
        println!("  Linux  : apt, pacman, dnf, emerge (binary/--usepkg only) + Flatpak");
        println!("  macOS  : brew (Homebrew)");
        println!("  Windows: winget");
        println!();
        println!("{}", "Examples:".bright_white().bold());
        println!("  hibrid {} vim", "-I".green());
        println!("  hibrid {} vim", "-Ia".green());
        println!("  hibrid {} firefox", "-Iq".green());
        println!("  hibrid {} package", "-R".red());
        println!("  hibrid {} spotify", "-If".bright_magenta());
        println!("  hibrid {}", "-U".yellow());
        println!("  hibrid {} vim", "-U".yellow());
        println!("  hibrid {}", "-L".cyan());
        return;
    }

    let system = detect_system();

    let filtered: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

    let action = match parse_action(filtered.get(0).unwrap_or(&"")) {
        Some(a) => a,
        None => {
            println!("{}", "Invalid command".red());
            return;
        }
    };

    // Search handling
    if matches!(action, Action::Search | Action::SearchFlatpak) {
        let packages: Vec<&str> = filtered.get(1..).unwrap_or(&[]).to_vec();
        if packages.is_empty() {
            println!("{}", "No package given".red());
            exit(1);
        }

        let package = packages[0];

        if let Action::SearchFlatpak = action {
            if system == System::Linux {
                match search_package_flatpak(package) {
                    Some(result) => println!("{}", format_search_box(package, &result).bright_magenta()),
                    None => println!("{}", format!("{}: Package not found", package).red()),
                }
            } else {
                println!("{}", "Flatpak search only available on Linux".red());
            }
        } else {
            match system {
                System::Linux => {
                    match detect_linux_package_manager() {
                        Some(manager) => {
                            match search_package_linux(package, &manager) {
                                Some(result) => println!("{}", format_search_box(package, &result).bright_cyan()),
                                None => println!("{}", format!("{}: Package not found", package).red()),
                            }
                        }
                        None => println!("{}", "No supported package manager found".red()),
                    }
                }
                System::MacOS => {
                    match detect_macos_package_manager() {
                        Some(manager) => {
                            match search_package_linux(package, &manager) {
                                Some(result) => println!("{}", format_search_box(package, &result).bright_cyan()),
                                None => println!("{}", format!("{}: Package not found", package).red()),
                            }
                        }
                        None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
                    }
                }
                System::Windows => {
                    println!("{}", "Search not yet supported for Windows".yellow());
                }
                System::Unknown => {
                    println!("{}", "Unsupported system".red());
                }
            }
        }
        return;
    }

    // List handling
    if matches!(action, Action::List | Action::ListFlatpak) {
        if system == System::Linux {
            if let Action::ListFlatpak = action {
                let (status, _) = run_command_with_output_detailed("flatpak", &["list", "--app"], "flatpak", true);
            } else {
                match detect_linux_package_manager() {
                    Some(manager) => {
                        let (status, _) = run_command_with_output_detailed("sudo", &{
                            let mut v = vec![manager.program];
                            v.extend(manager.list_args);
                            v
                        }, manager.program, true);
                    }
                    None => println!("{}", "No supported package manager found".red()),
                }
            }
        } else if system == System::MacOS {
            if let Action::ListFlatpak = action {
                println!("{}", "Flatpak is not available on macOS".red());
            } else {
                match detect_macos_package_manager() {
                    Some(manager) => {
                        let (_, _) = run_command_with_output_detailed(manager.program, &{
                            let mut v = manager.list_args.to_vec();
                            v
                        }, manager.program, true);
                    }
                    None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
                }
            }
        } else if system == System::Windows {
            println!("{}", "List not yet supported for Windows".yellow());
        } else {
            println!("{}", "Unsupported system".red());
        }
        return;
    }

    let packages: Vec<&str> = filtered.get(1..).unwrap_or(&[]).to_vec();

    // Update handling (packages optional - empty means update all)
    if matches!(action, Action::Update | Action::UpdateQuiet | Action::UpdateAutoinstall | Action::UpdateAutoinstallQuiet | Action::UpdateFlatpak | Action::UpdateFlatpakQuiet | Action::UpdateAutoinstallFlatpak | Action::UpdateAutoinstallFlatpakQuiet) {
        if system == System::Linux {
            let is_quiet = matches!(action, Action::UpdateQuiet | Action::UpdateAutoinstallQuiet | Action::UpdateFlatpakQuiet | Action::UpdateAutoinstallFlatpakQuiet);
            let skip_confirm = is_autoinstall(action);

            if let Action::UpdateFlatpak | Action::UpdateFlatpakQuiet | Action::UpdateAutoinstallFlatpak | Action::UpdateAutoinstallFlatpakQuiet = action {
                // Flatpak update
                if packages.is_empty() {
                    // Update all flatpaks
                    println!("{}", format_box_multiple("Update Flatpak", vec![("All installed flatpaks".to_string(), String::new(), String::new())]).bright_magenta());
                    if !skip_confirm && !ask_update_confirmation() {
                        println!("{}", "Update cancelled".yellow());
                        return;
                    }
                    let (status, _) = run_command_with_output_detailed("flatpak", &["update", "-y"], "flatpak", !is_quiet);
                    print_result(action, status, "");
                } else {
                    // Update specific flatpak(s)
                    let packages_info: Vec<(String, String, String)> = packages.iter()
                        .map(|p| (p.to_string(), String::new(), String::new()))
                        .collect();
                    println!("{}", format_box_multiple("Update Flatpak", packages_info).bright_magenta());
                    
                    for package in &packages {
                        if !skip_confirm && !ask_update_confirmation() {
                            println!("{}", "Update cancelled".yellow());
                            return;
                        }
                        let (status, _) = run_command_with_output_detailed("flatpak", &["update", "-y", package], "flatpak", !is_quiet);
                        print_result(action, status, "");
                    }
                }
            } else {
                // System package manager update
                match detect_linux_package_manager() {
                    Some(manager) => {
                        if packages.is_empty() {
                            // Update system
                            println!("{}", format_box_multiple("Update", vec![("All packages".to_string(), String::new(), String::new())]).bright_cyan());
                            if !skip_confirm && !ask_update_confirmation() {
                                println!("{}", "Update cancelled".yellow());
                                return;
                            }
                            let (status, _) = run_command_with_output_detailed("sudo", &{
                                let mut v = vec![manager.program];
                                v.extend(manager.update_args);
                                v
                            }, manager.program, !is_quiet);
                            print_result(action, status, "");
                        } else {
                            // Update specific package(s)
                            let packages_info: Vec<(String, String, String)> = packages.iter()
                                .map(|p| (p.to_string(), String::new(), String::new()))
                                .collect();
                            println!("{}", format_box_multiple("Update", packages_info).bright_cyan());
                            
                            for package in &packages {
                                if !skip_confirm && !ask_update_confirmation() {
                                    println!("{}", "Update cancelled".yellow());
                                    return;
                                }
                                let (status, _) = run_command_with_output_detailed("sudo", &{
                                    let mut v = vec![manager.program];
                                    let mut base = manager.update_single_args.to_vec();
                                    base.push(package);
                                    v.extend(base);
                                    v
                                }, manager.program, !is_quiet);
                                print_result(action, status, "");
                            }
                        }
                    }
                    None => println!("{}", "No supported package manager found".red()),
                }
            }
        } else if system == System::Windows {
            println!("{}", "Update not yet supported for Windows".yellow());
        } else if system == System::MacOS {
            let is_quiet = matches!(action, Action::UpdateQuiet | Action::UpdateAutoinstallQuiet | Action::UpdateFlatpakQuiet | Action::UpdateAutoinstallFlatpakQuiet);
            let skip_confirm = is_autoinstall(action);

            if matches!(action, Action::UpdateFlatpak | Action::UpdateFlatpakQuiet | Action::UpdateAutoinstallFlatpak | Action::UpdateAutoinstallFlatpakQuiet) {
                println!("{}", "Flatpak is not available on macOS".red());
            } else {
                match detect_macos_package_manager() {
                    Some(manager) => {
                        // brew update refreshes formulae; brew upgrade upgrades packages
                        // Run `brew update` first, then `brew upgrade`
                        if packages.is_empty() {
                            println!("{}", format_box_multiple("Update", vec![("All brew packages".to_string(), String::new(), String::new())]).bright_cyan());
                            if !skip_confirm && !ask_update_confirmation() {
                                println!("{}", "Update cancelled".yellow());
                                return;
                            }
                            // sync formula list
                            let _ = run_command_with_output_detailed(manager.program, &["update"], manager.program, !is_quiet);
                            // upgrade all
                            let (status, _) = run_command_with_output_detailed(manager.program, manager.update_args, manager.program, !is_quiet);
                            print_result(action, status, "");
                        } else {
                            let packages_info: Vec<(String, String, String)> = packages.iter()
                                .map(|p| (p.to_string(), "homebrew".to_string(), String::new()))
                                .collect();
                            println!("{}", format_box_multiple("Update", packages_info).bright_cyan());
                            for package in &packages {
                                if !skip_confirm && !ask_update_confirmation() {
                                    println!("{}", "Update cancelled".yellow());
                                    return;
                                }
                                let (status, _) = run_command_with_output_detailed(manager.program, &{
                                    let mut base = manager.update_single_args.to_vec();
                                    base.push(package);
                                    base
                                }, manager.program, !is_quiet);
                                print_result(action, status, "");
                            }
                        }
                    }
                    None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
                }
            }
        } else {
            println!("{}", "Unsupported system".red());
        }
        return;
    }

    if packages.is_empty() {
        println!("{}", "No package given".red());
        exit(1);
    }

    // Flatpak handling
    if matches!(action, Action::InstallFlatpak | Action::InstallFlatpakQuiet | Action::InstallAutoinstallFlatpak | Action::InstallAutoinstallFlatpakQuiet | Action::RemoveFlatpak | Action::RemoveFlatpakQuiet | Action::RemoveAutoinstallFlatpak | Action::RemoveAutoinstallFlatpakQuiet) && system == System::Linux {
        let is_quiet = matches!(action, Action::InstallFlatpakQuiet | Action::RemoveFlatpakQuiet | Action::InstallAutoinstallFlatpakQuiet | Action::RemoveAutoinstallFlatpakQuiet);
        let skip_confirm = is_autoinstall(action);

        match action {
            Action::InstallFlatpak | Action::InstallFlatpakQuiet | Action::InstallAutoinstallFlatpak | Action::InstallAutoinstallFlatpakQuiet => {
                let mut all_valid = true;
                let mut packages_info = Vec::new();
                let mut full_app_ids = Vec::new();

                for package in &packages {
                    let (full_app_id, size) = match fuzzy_match_flatpak_with_size(package) {
                        Some((id, sz)) => (id, sz),
                        None => {
                            println!("{}", format!("{}: Package not found", package).red());
                            all_valid = false;
                            continue;
                        }
                    };
                    packages_info.push((package.to_string(), "flathub".to_string(), size));
                    full_app_ids.push(full_app_id);
                }

                if !all_valid || packages_info.is_empty() {
                    return;
                }

                let title = if is_quiet { "Install Flatpak Quiet" } else { "Install Flatpak" };
                println!("{}", format_box_multiple(title, packages_info).bright_magenta());

                if !skip_confirm && !ask_confirmation() {
                    println!("{}", "Installation cancelled".yellow());
                    return;
                }

                for full_app_id in full_app_ids {
                    let (status, _) = run_command_with_output_detailed("flatpak", &["install", "-y", "flathub", &full_app_id], "flatpak", !is_quiet);
                    print_result(action, status, "");
                }
            }
            Action::RemoveFlatpak | Action::RemoveFlatpakQuiet | Action::RemoveAutoinstallFlatpak | Action::RemoveAutoinstallFlatpakQuiet => {
                let mut all_valid = true;
                let mut packages_info = Vec::new();
                let mut app_ids = Vec::new();

                for package in &packages {
                    let mut app_id = package.to_string();

                    if !package.contains(".") {
                        if let Some(id) = fuzzy_match_flatpak(package) {
                            app_id = id;
                        }
                    }

                    // Check if the flatpak is actually installed
                    if !is_flatpak_installed(&app_id) {
                        println!("{}", format!("{}: Package not installed or doesn't exist", package).red());
                        all_valid = false;
                        continue;
                    }

                    packages_info.push((package.to_string(), String::new(), String::new()));
                    app_ids.push(app_id);
                }

                if !all_valid || packages_info.is_empty() {
                    return;
                }

                let title = if is_quiet { "Remove Flatpak Quiet" } else { "Remove Flatpak" };
                println!("{}", format_box_multiple(title, packages_info).bright_magenta());

                if !skip_confirm && !ask_removal_confirmation() {
                    println!("{}", "Removal cancelled".yellow());
                    return;
                }

                for app_id in app_ids {
                    let (status, _) = run_command_with_output_detailed("flatpak", &["uninstall", "-y", &app_id], "flatpak", !is_quiet);
                    print_result(action, status, "");
                }
            }
            _ => {}
        }
        return;
    }

    let (_success, _info) = match system {
        System::Windows => {
            let winget = PackageManager {
                program: "winget",
                install_args: &["install", "--exact"],
                remove_args: &["uninstall", "--exact"],
                update_args: &["upgrade"],
                update_single_args: &["upgrade", "--exact"],
                list_args: &["list"],
                search_args: &["search"],
            };

            for package in &packages {
                let (status, info) = winget.run(action, package);
                print_result(action, status, &info);
            }
            (true, String::new())
        }

        System::Linux => {
            match detect_linux_package_manager() {
                Some(manager) => {
                    let is_quiet = matches!(action, Action::InstallQuiet | Action::RemoveQuiet | Action::InstallAutoinstallQuiet | Action::RemoveAutoinstallQuiet);
                    let skip_confirm = is_autoinstall(action);

                    match action {
                        Action::Install | Action::InstallQuiet | Action::InstallAutoinstall | Action::InstallAutoinstallQuiet => {
                            let mut all_valid = true;
                            let mut packages_info = Vec::new();

                            for package in &packages {
                                let (repo, size) = manager.search_info(package);
                                if size.is_empty() {
                                    println!("{}", format!("{}: Package not found", package).red());
                                    all_valid = false;
                                    continue;
                                }
                                packages_info.push((package.to_string(), repo, size));
                            }

                            if !all_valid {
                                return;
                            }

                            println!("{}", format_box_multiple("Install", packages_info).bright_cyan());

                            if !skip_confirm && !ask_confirmation() {
                                println!("{}", "Installation cancelled".yellow());
                                return;
                            }

                            for package in &packages {
                                let (status, _) = run_command_with_output_detailed("sudo", &{
                                    let mut v = vec![manager.program];
                                    let mut base = manager.install_args.to_vec();
                                    base.push(package);
                                    v.extend(base);
                                    v
                                }, manager.program, !is_quiet);
                                print_result(action, status, "");
                            }
                        }
                        Action::Remove | Action::RemoveQuiet | Action::RemoveAutoinstall | Action::RemoveAutoinstallQuiet => {
                            let mut all_valid = true;
                            let mut packages_info = Vec::new();

                            for package in &packages {
                                let (_, size) = get_installed_package_info(manager.program, package, manager.program);
                                if size.is_empty() {
                                    println!("{}", format!("{}: Package not installed or doesn't exist", package).red());
                                    all_valid = false;
                                    continue;
                                }
                                packages_info.push((package.to_string(), String::new(), size));
                            }

                            if !all_valid {
                                return;
                            }

                            println!("{}", format_box_multiple("Remove", packages_info).bright_red());

                            if !skip_confirm && !ask_removal_confirmation() {
                                println!("{}", "Removal cancelled".yellow());
                                return;
                            }

                            for package in &packages {
                                let (status, _) = run_command_with_output_detailed("sudo", &{
                                    let mut v = vec![manager.program];
                                    let mut base = manager.remove_args.to_vec();
                                    base.push(package);
                                    v.extend(base);
                                    v
                                }, manager.program, !is_quiet);
                                print_result(action, status, "");
                            }
                        }
                        _ => {}
                    }
                    (true, String::new())
                }
                None => {
                    println!("{}", "No supported package manager found".red());
                    (false, String::new())
                }
            }
        }

        System::Unknown => {
            println!("{}", "Unsupported system".red());
            (false, String::new())
        }

        System::MacOS => {
            match detect_macos_package_manager() {
                Some(manager) => {
                    let is_quiet = matches!(action, Action::InstallQuiet | Action::RemoveQuiet | Action::InstallAutoinstallQuiet | Action::RemoveAutoinstallQuiet);
                    let skip_confirm = is_autoinstall(action);

                    // Flatpak actions are not supported on macOS
                    if matches!(action,
                        Action::InstallFlatpak | Action::InstallFlatpakQuiet |
                        Action::InstallAutoinstallFlatpak | Action::InstallAutoinstallFlatpakQuiet |
                        Action::RemoveFlatpak | Action::RemoveFlatpakQuiet |
                        Action::RemoveAutoinstallFlatpak | Action::RemoveAutoinstallFlatpakQuiet)
                    {
                        println!("{}", "Flatpak is not available on macOS".red());
                        return (false, String::new());
                    }

                    match action {
                        Action::Install | Action::InstallQuiet | Action::InstallAutoinstall | Action::InstallAutoinstallQuiet => {
                            let mut all_valid = true;
                            let mut packages_info = Vec::new();

                            for package in &packages {
                                let (repo, size) = manager.search_info(package);
                                if size.is_empty() {
                                    println!("{}", format!("{}: Package not found", package).red());
                                    all_valid = false;
                                    continue;
                                }
                                packages_info.push((package.to_string(), repo, size));
                            }

                            if !all_valid {
                                return (false, String::new());
                            }

                            println!("{}", format_box_multiple("Install", packages_info).bright_cyan());

                            if !skip_confirm && !ask_confirmation() {
                                println!("{}", "Installation cancelled".yellow());
                                return (false, String::new());
                            }

                            // brew does not need sudo
                            for package in &packages {
                                let (status, _) = run_command_with_output_detailed(manager.program, &{
                                    let mut base = manager.install_args.to_vec();
                                    base.push(package);
                                    base
                                }, manager.program, !is_quiet);
                                print_result(action, status, "");
                            }
                        }
                        Action::Remove | Action::RemoveQuiet | Action::RemoveAutoinstall | Action::RemoveAutoinstallQuiet => {
                            let mut all_valid = true;
                            let mut packages_info = Vec::new();

                            for package in &packages {
                                let (_, size) = get_installed_package_info(manager.program, package, manager.program);
                                if size.is_empty() {
                                    println!("{}", format!("{}: Package not installed or doesn't exist", package).red());
                                    all_valid = false;
                                    continue;
                                }
                                packages_info.push((package.to_string(), String::new(), size));
                            }

                            if !all_valid {
                                return (false, String::new());
                            }

                            println!("{}", format_box_multiple("Remove", packages_info).bright_red());

                            if !skip_confirm && !ask_removal_confirmation() {
                                println!("{}", "Removal cancelled".yellow());
                                return (false, String::new());
                            }

                            // brew does not need sudo
                            for package in &packages {
                                let (status, _) = run_command_with_output_detailed(manager.program, &{
                                    let mut base = manager.remove_args.to_vec();
                                    base.push(package);
                                    base
                                }, manager.program, !is_quiet);
                                print_result(action, status, "");
                            }
                        }
                        _ => {}
                    }
                    (true, String::new())
                }
                None => {
                    println!("{}", "No package manager found (is Homebrew installed?)".red());
                    (false, String::new())
                }
            }
        }
    };
}



fn format_box(title: &str, package: &str, repo: &str, size: &str) -> String {
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

fn format_box_multiple(title: &str, packages_info: Vec<(String, String, String)>) -> String {
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

fn print_result(action: Action, success: bool, _info: &str) {
    match (action, success) {
        (Action::Install, true) => println!("{}", "Install finished".green()),
        (Action::Install, false) => println!("{}", "Install failed".red()),
        (Action::InstallQuiet, true) => println!("{}", "Install finished".green()),
        (Action::InstallQuiet, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstall, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstall, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstallQuiet, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstallQuiet, false) => println!("{}", "Install failed".red()),
        (Action::InstallFlatpak, true) => println!("{}", "Install finished".green()),
        (Action::InstallFlatpak, false) => println!("{}", "Install failed".red()),
        (Action::InstallFlatpakQuiet, true) => println!("{}", "Install finished".green()),
        (Action::InstallFlatpakQuiet, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstallFlatpak, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstallFlatpak, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstallFlatpakQuiet, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstallFlatpakQuiet, false) => println!("{}", "Install failed".red()),
        (Action::Remove, true) => println!("{}", "Removal finished".green()),
        (Action::Remove, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveQuiet, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveQuiet, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstall, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstall, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstallQuiet, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstallQuiet, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveFlatpak, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveFlatpak, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveFlatpakQuiet, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveFlatpakQuiet, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstallFlatpak, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstallFlatpak, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstallFlatpakQuiet, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstallFlatpakQuiet, false) => println!("{}", "Removal failed".red()),
        (Action::Update, true) => println!("{}", "Update finished".green()),
        (Action::Update, false) => println!("{}", "Update failed".red()),
        (Action::UpdateQuiet, true) => println!("{}", "Update finished".green()),
        (Action::UpdateQuiet, false) => println!("{}", "Update failed".red()),
        (Action::UpdateAutoinstall, true) => println!("{}", "Update finished".green()),
        (Action::UpdateAutoinstall, false) => println!("{}", "Update failed".red()),
        (Action::UpdateAutoinstallQuiet, true) => println!("{}", "Update finished".green()),
        (Action::UpdateAutoinstallQuiet, false) => println!("{}", "Update failed".red()),
        (Action::UpdateFlatpak, true) => println!("{}", "Update finished".green()),
        (Action::UpdateFlatpak, false) => println!("{}", "Update failed".red()),
        (Action::UpdateFlatpakQuiet, true) => println!("{}", "Update finished".green()),
        (Action::UpdateFlatpakQuiet, false) => println!("{}", "Update failed".red()),
        (Action::UpdateAutoinstallFlatpak, true) => println!("{}", "Update finished".green()),
        (Action::UpdateAutoinstallFlatpak, false) => println!("{}", "Update failed".red()),
        (Action::UpdateAutoinstallFlatpakQuiet, true) => println!("{}", "Update finished".green()),
        (Action::UpdateAutoinstallFlatpakQuiet, false) => println!("{}", "Update failed".red()),
        _ => {}
    }
}
