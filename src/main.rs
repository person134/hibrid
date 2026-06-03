use std::env;
use std::process::{Command, Stdio};
use std::process::exit;
use std::io::{self, Write};
use colored::*;

/// Supported operating systems
#[derive(Debug, PartialEq)]
enum System {
    Windows,
    Linux,
    Unknown,
}

/// Supported actions
#[derive(Debug, Clone, Copy)]
enum Action {
    Install,
    InstallDetailed,
    InstallAutoinstall,
    InstallAutoinstallDetailed,
    InstallFlatpak,
    InstallFlatpakDetailed,
    InstallAutoinstallFlatpak,
    InstallAutoinstallFlatpakDetailed,
    Remove,
    RemoveDetailed,
    RemoveAutoinstall,
    RemoveAutoinstallDetailed,
    RemoveFlatpak,
    RemoveFlatpakDetailed,
    RemoveAutoinstallFlatpak,
    RemoveAutoinstallFlatpakDetailed,
    Version,
}

/// Represents a package manager and its commands
struct PackageManager {
    program: &'static str,
    install_args: &'static [&'static str],
    remove_args: &'static [&'static str],
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
        // For normal mode, capture output
        if program == "sudo" {
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
        }
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
        _ => {}
    }
    String::new()
}

/// Fuzzy match flatpak app ID
fn fuzzy_match_flatpak(query: &str) -> Option<String> {
    match Command::new("flatpak").args(&["search", query]).output() {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                // Skip header line and empty lines
                if line.contains("Name") || line.trim().is_empty() {
                    continue;
                }
                // Look for app ID pattern (org.* or com.*)
                for part in line.split_whitespace() {
                    if (part.starts_with("org.") || part.starts_with("com.")) && part.contains('.') {
                        return Some(part.to_string());
                    }
                }
            }
            None
        }
        Err(_) => None,
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

/// Format action for display
fn format_action(action: Action) -> &'static str {
    match action {
        Action::Install => "Install",
        Action::InstallDetailed => "Install Detailed",
        Action::InstallAutoinstall => "Install Autoinstall",
        Action::InstallAutoinstallDetailed => "Install Autoinstall Detailed",
        Action::InstallFlatpak => "Install Flatpak",
        Action::InstallFlatpakDetailed => "Install Flatpak Detailed",
        Action::InstallAutoinstallFlatpak => "Install Flatpak Autoinstall",
        Action::InstallAutoinstallFlatpakDetailed => "Install Flatpak Autoinstall Detailed",
        Action::Remove => "Remove",
        Action::RemoveDetailed => "Remove Detailed",
        Action::RemoveAutoinstall => "Remove Autoinstall",
        Action::RemoveAutoinstallDetailed => "Remove Autoinstall Detailed",
        Action::RemoveFlatpak => "Remove Flatpak",
        Action::RemoveFlatpakDetailed => "Remove Flatpak Detailed",
        Action::RemoveAutoinstallFlatpak => "Remove Flatpak Autoinstall",
        Action::RemoveAutoinstallFlatpakDetailed => "Remove Flatpak Autoinstall Detailed",
        Action::Version => "Version",
    }
}

/// Check if a command exists (silently)
fn command_exists(program: &str) -> bool {
    match Command::new("which")
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
            search_args: &["cache", "show"],
        },
        PackageManager {
            program: "pacman",
            install_args: &["-S", "--noconfirm"],
            remove_args: &["-R", "--noconfirm"],
            search_args: &["-Si"],
        },
        PackageManager {
            program: "dnf",
            install_args: &["install", "-y"],
            remove_args: &["remove", "-y"],
            search_args: &["info"],
        },
    ];

    for manager in managers {
        if command_exists(manager.program) {
            return Some(manager);
        }
    }

    None
}

fn parse_action(flag: &str) -> Option<Action> {
    if !flag.starts_with('-') || flag.len() < 2 {
        return None;
    }
    
    let flag_chars = &flag[1..]; // Remove the leading '-'
    let base = flag_chars.chars().next().unwrap();
    
    // Handle V separately - it cannot have modifiers
    if base == 'V' {
        return if flag_chars.len() == 1 {
            Some(Action::Version)
        } else {
            None // -V with any modifiers is invalid
        };
    }
    
    let modifiers = &flag_chars[1..];
    
    // Check for presence of modifier characters in any order
    let has_a = modifiers.contains('a');
    let has_d = modifiers.contains('d');
    let has_f = modifiers.contains('f');
    
    match (base, has_a, has_d, has_f) {
        ('I', false, false, false) => Some(Action::Install),
        ('I', false, true, false) => Some(Action::InstallDetailed),
        ('I', true, false, false) => Some(Action::InstallAutoinstall),
        ('I', true, true, false) => Some(Action::InstallAutoinstallDetailed),
        ('I', false, false, true) => Some(Action::InstallFlatpak),
        ('I', false, true, true) => Some(Action::InstallFlatpakDetailed),
        ('I', true, false, true) => Some(Action::InstallAutoinstallFlatpak),
        ('I', true, true, true) => Some(Action::InstallAutoinstallFlatpakDetailed),
        ('R', false, false, false) => Some(Action::Remove),
        ('R', false, true, false) => Some(Action::RemoveDetailed),
        ('R', true, false, false) => Some(Action::RemoveAutoinstall),
        ('R', true, true, false) => Some(Action::RemoveAutoinstallDetailed),
        ('R', false, false, true) => Some(Action::RemoveFlatpak),
        ('R', false, true, true) => Some(Action::RemoveFlatpakDetailed),
        ('R', true, false, true) => Some(Action::RemoveAutoinstallFlatpak),
        ('R', true, true, true) => Some(Action::RemoveAutoinstallFlatpakDetailed),
        _ => None,
    }
}

/// Check if an action is in autoinstall mode
fn is_autoinstall(action: Action) -> bool {
    matches!(
        action,
        Action::InstallAutoinstall
            | Action::InstallAutoinstallDetailed
            | Action::InstallAutoinstallFlatpak
            | Action::InstallAutoinstallFlatpakDetailed
            | Action::RemoveAutoinstall
            | Action::RemoveAutoinstallDetailed
            | Action::RemoveAutoinstallFlatpak
            | Action::RemoveAutoinstallFlatpakDetailed
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("{}", "╔════════════════════════════════════════════════════════════╗".bright_cyan());
        println!("{}", "║          Hibrid Package Manager Wrapper v1.0              ║".bright_cyan());
        println!("{}", "╚════════════════════════════════════════════════════════════╝".bright_cyan());
        println!();
        println!("{}", "Usage: hibrid [-I|-R|-V][a][d][f] pkg".bright_white().bold());
        println!();
        println!("  {} Install package", "-I".green().bold());
        println!("  {} Remove package", "-R".red().bold());
        println!("  {} Show version", "-V".yellow().bold());
        println!();
        println!("{}", "Modifiers:".bright_white().bold());
        println!("  {} Autoinstall (skip confirmation)", "a".bright_yellow());
        println!("  {} Detailed output", "d".bright_yellow());
        println!("  {} Use Flatpak", "f".bright_magenta());
        println!();
        println!("{}", "Examples:".bright_white().bold());
        println!("  hibrid {} vim", "-I".green());
        println!("  hibrid {} vim", "-Ia".green());
        println!("  hibrid {} package", "-R".red());
        println!("  hibrid {} spotify", "-If".bright_magenta());
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

    if let Action::Version = action {
        println!("{}", "╔════════════════════════════════════════════════════════════╗".bright_cyan());
        println!("{} {}", "║".bright_cyan(), "Hibrid package manager wrapper v1.0".bright_cyan().bold());
        println!("{}", "╚════════════════════════════════════════════════════════════╝".bright_cyan());
        return;
    }

    let package = filtered.get(1).unwrap_or(&"");

    if package.is_empty() {
        println!("{}", "No package given".red());
        exit(1);
    }

    // Flatpak handling
    if matches!(action, Action::InstallFlatpak | Action::InstallFlatpakDetailed | Action::InstallAutoinstallFlatpak | Action::InstallAutoinstallFlatpakDetailed | Action::RemoveFlatpak | Action::RemoveFlatpakDetailed | Action::RemoveAutoinstallFlatpak | Action::RemoveAutoinstallFlatpakDetailed) && system == System::Linux {
        println!("{}", format!("╭─ {} ─╮", format_action(action)).bright_magenta().bold());

        let is_detailed = matches!(action, Action::InstallFlatpakDetailed | Action::RemoveFlatpakDetailed | Action::InstallAutoinstallFlatpakDetailed | Action::RemoveAutoinstallFlatpakDetailed);
        let skip_confirm = is_autoinstall(action);

        match action {
            Action::InstallFlatpak | Action::InstallFlatpakDetailed | Action::InstallAutoinstallFlatpak | Action::InstallAutoinstallFlatpakDetailed => {
                // Fuzzy match the package name to get the full app ID
                let full_app_id = match fuzzy_match_flatpak(package) {
                    Some(id) => id,
                    None => {
                        println!("{}", "Package not found".red());
                        return;
                    }
                };

                println!("flatpak found {} in flathub repository.", package.bright_green().bold());

                if !skip_confirm && !ask_confirmation() {
                    println!("{}", "Installation cancelled".yellow());
                    return;
                }

                let (status, _) = run_command_with_output_detailed("flatpak", &["install", "-y", "-d", "flathub", &full_app_id], "flatpak", is_detailed);
                print_result(action, status, "");
            }
            Action::RemoveFlatpak | Action::RemoveFlatpakDetailed | Action::RemoveAutoinstallFlatpak | Action::RemoveAutoinstallFlatpakDetailed => {
                // First try with the provided package name, otherwise use fuzzy search result
                let mut app_id = package.to_string();

                // If package doesn't look like an app ID, try fuzzy match
                if !package.contains(".") {
                    if let Some(id) = fuzzy_match_flatpak(package) {
                        app_id = id;
                    }
                }

                println!("{} {}", package.bright_red().bold(), "will be removed.".bright_white());

                if !skip_confirm && !ask_removal_confirmation() {
                    println!("{}", "Removal cancelled".yellow());
                    return;
                }

                let (status, _) = run_command_with_output_detailed("flatpak", &["uninstall", "-y", &app_id], "flatpak", is_detailed);
                print_result(action, status, "");
            }
            _ => {}
        }
        return;
    }

    let header_color = if matches!(action, Action::Install | Action::InstallDetailed | Action::InstallAutoinstall | Action::InstallAutoinstallDetailed | Action::InstallFlatpak | Action::InstallFlatpakDetailed | Action::InstallAutoinstallFlatpak | Action::InstallAutoinstallFlatpakDetailed) {
        format!("╭─ {} ─╮", format_action(action)).bright_green()
    } else {
        format!("╭─ {} ─╮", format_action(action)).bright_red()
    };
    println!("{}", header_color.bold());

    let (success, info) = match system {
        System::Windows => {
            let winget = PackageManager {
                program: "winget",
                install_args: &["install", "--exact"],
                remove_args: &["uninstall", "--exact"],
                search_args: &["search"],
            };
            winget.run(action, package)
        }

        System::Linux => {
            match detect_linux_package_manager() {
                Some(manager) => {
                    let is_detailed = matches!(action, Action::InstallDetailed | Action::RemoveDetailed | Action::InstallAutoinstallDetailed | Action::RemoveAutoinstallDetailed);
                    let skip_confirm = is_autoinstall(action);

                    match action {
                        Action::Install | Action::InstallDetailed | Action::InstallAutoinstall | Action::InstallAutoinstallDetailed => {
                            // Show package info and ask for confirmation before installing
                            let (repo, size) = manager.search_info(package);

                            // Check if package was found
                            if size.is_empty() {
                                println!("{}", "Package not found".red());
                                return;
                            }

                            if !repo.is_empty() {
                                println!("{} {} {}", package.bright_green().bold(), "found in".bright_white(), format!("\"{}\"", repo).bright_yellow());
                            } else {
                                println!("{} {}", package.bright_green().bold(), "found".bright_white());
                            }
                            if !size.is_empty() {
                                println!("Total size: {}", size.bright_yellow());
                            }

                            if !skip_confirm && !ask_confirmation() {
                                println!("{}", "Installation cancelled".yellow());
                                return;
                            }
                        }
                        Action::Remove | Action::RemoveDetailed | Action::RemoveAutoinstall | Action::RemoveAutoinstallDetailed => {
                            // Show removal info and ask for confirmation before removing
                            let (_, size) = manager.search_info(package);

                            // Check if package exists/is installed
                            if size.is_empty() {
                                println!("{}", "Package not installed or doesn't exist".red());
                                return;
                            }

                            println!("{} {}", package.bright_red().bold(), "will be removed.".bright_white());
                            if !size.is_empty() {
                                println!("Total removed size: {}", size.bright_yellow());
                            }

                            if !skip_confirm && !ask_removal_confirmation() {
                                println!("{}", "Removal cancelled".yellow());
                                return;
                            }
                        }
                        _ => {}
                    }

                    let (status, _) = run_command_with_output_detailed("sudo", &{
                        let mut v = vec![manager.program];
                        let mut base = match action {
                            Action::Install | Action::InstallDetailed => manager.install_args.to_vec(),
                            Action::Remove | Action::RemoveDetailed => manager.remove_args.to_vec(),
                            _ => vec![],
                        };
                        base.push(package);
                        v.extend(base);
                        v
                    }, manager.program, is_detailed);
                    (status, String::new())
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
    };

    print_result(action, success, &info);
}

fn print_result(action: Action, success: bool, _info: &str) {
    match (action, success) {
        (Action::Install, true) => println!("{}", "Install finished".green()),
        (Action::Install, false) => println!("{}", "Install failed".red()),
        (Action::InstallDetailed, true) => println!("{}", "Install finished".green()),
        (Action::InstallDetailed, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstall, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstall, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstallDetailed, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstallDetailed, false) => println!("{}", "Install failed".red()),
        (Action::InstallFlatpak, true) => println!("{}", "Install finished".green()),
        (Action::InstallFlatpak, false) => println!("{}", "Install failed".red()),
        (Action::InstallFlatpakDetailed, true) => println!("{}", "Install finished".green()),
        (Action::InstallFlatpakDetailed, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstallFlatpak, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstallFlatpak, false) => println!("{}", "Install failed".red()),
        (Action::InstallAutoinstallFlatpakDetailed, true) => println!("{}", "Install finished".green()),
        (Action::InstallAutoinstallFlatpakDetailed, false) => println!("{}", "Install failed".red()),
        (Action::Remove, true) => println!("{}", "Removal finished".green()),
        (Action::Remove, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveDetailed, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveDetailed, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstall, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstall, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstallDetailed, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstallDetailed, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveFlatpak, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveFlatpak, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveFlatpakDetailed, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveFlatpakDetailed, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstallFlatpak, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstallFlatpak, false) => println!("{}", "Removal failed".red()),
        (Action::RemoveAutoinstallFlatpakDetailed, true) => println!("{}", "Removal finished".green()),
        (Action::RemoveAutoinstallFlatpakDetailed, false) => println!("{}", "Removal failed".red()),
        _ => {}
    }
}
