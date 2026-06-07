mod action;
mod backend;
mod runner;
mod search;
mod ui;

use std::env;
use std::io::{self, Write};
use std::process::exit;
use colored::*;

use action::{Action, Flags, parse_arguments};
use backend::{System, detect_system, detect_linux_package_manager, detect_macos_package_manager, detect_aur_helper, requires_sudo, command_exists, PackageManager};
use runner::run_command_with_output_detailed;
use search::{search_info, search_package_linux, search_package_flatpak, get_installed_package_info, fuzzy_match_flatpak, fuzzy_match_flatpak_with_size, is_flatpak_installed};
use ui::{format_box_multiple, format_search_box, print_result, ask_confirmation, ask_removal_confirmation, ask_update_confirmation, ask_flatpak_install};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        return;
    }

    if args[1] == "-V" || args[1] == "--version" {
        println!("Hibrid {}", env!("CARGO_PKG_VERSION"));
        return;
    }

    if args[1] == "-h" || args[1] == "--help" {
        print_help();
        return;
    }

    let system = detect_system();
    let filtered: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

    let (action, flags, packages) = parse_arguments(&filtered)
        .unwrap_or_else(|| {
            println!("{}", "Invalid command".red());
            exit(1);
        });

    match action {
        Action::Search => handle_search(system, flags, &packages),
        Action::List => handle_list(system, flags),
        Action::Update => handle_update(system, flags, &packages),
        Action::Install => handle_install(system, flags, &packages),
        Action::Remove => handle_remove(system, flags, &packages),
    }
}

fn print_help() {
    println!("{}", "╔════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║              Hibrid Package Manager Wrapper               ║".bright_cyan());
    println!("{}", "╚════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("{}", "Usage:".bright_white().bold());
    println!("  hibrid <command> [modifiers] [packages...]");
    println!("  hibrid -<FLAG><modifiers> [packages...] [modifiers]");
    println!("  hibrid -h | --help");
    println!("  hibrid -V | --version");
    println!();
    println!("{}", "Commands:".bright_white().bold());
    println!("  {} (or {}) Install package(s)", "install".green().bold(), "-I".green().bold());
    println!("  {} (or {}) Remove package(s)", "remove".red().bold(), "-R".red().bold());
    println!("  {} (or {}) Update system or package(s)", "update".yellow().bold(), "-U".yellow().bold());
    println!("  {} (or {}) List installed packages", "list".cyan().bold(), "-L".cyan().bold());
    println!("  {} (or {}) Search for packages", "search".bright_white().bold(), "-S".bright_white().bold());
    println!();
    println!("{}", "Modifiers:".bright_white().bold());
    println!("  -y, --yes      Skip confirmation prompts");
    println!("  -q, --quiet    Suppress detailed output");
    println!("  -f, --flatpak  Use Flatpak (Linux only)");
    println!("  -d, --dry-run  Preview without making changes");
    println!("  -V, --version  Show version");
    println!("  -h, --help     Show this help message");
    println!();
    println!("{}", "Supported backends:".bright_white().bold());
    println!("  Linux  : apt, pacman, dnf, emerge + Flatpak (AUR via yay/paru)");
    println!("  macOS  : Homebrew");
    println!("  Windows: winget");
    println!();
    println!("{}", "Examples:".bright_white().bold());
    println!("  hibrid install vim");
    println!("  hibrid -I vim");
    println!("  hibrid remove -y firefox");
    println!("  hibrid -R spotify -f");
    println!("  hibrid update");
    println!("  hibrid update vim");
    println!("  hibrid list");
    println!("  hibrid search git");
    println!("  hibrid -V");
}

fn ensure_flatpak_installed() -> bool {
    if command_exists("flatpak") {
        return true;
    }
    if !ask_flatpak_install() {
        return false;
    }
    let manager = match detect_linux_package_manager() {
        Some(m) => m,
        None => {
            println!("{}", "No package manager found to install Flatpak".red());
            return false;
        }
    };
    let mut args = vec![manager.program];
    args.extend(manager.install_args);
    args.push("flatpak");
    let (prog, cmd_args) = if requires_sudo(&manager) { ("sudo", args.as_slice()) } else { (manager.program, &args[1..]) };
    let (status, _) = run_command_with_output_detailed(prog, cmd_args, manager.program, true);
    status
}

fn ensure_aur_helper() -> Option<PackageManager> {
    if let Some(helper) = detect_aur_helper() {
        return Some(helper);
    }
    println!("{}", "This package is only available in the AUR.".yellow());
    print!("{} {} {} ", "?".bright_cyan().bold(), "Install yay (AUR helper) now?".bright_white(), "(Y/n):".bright_black());
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    if !input.trim().is_empty() && !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
        return None;
    }

    println!("{}", "Installing build dependencies (git, base-devel)...".yellow());
    let deps = ["pacman", "-S", "--noconfirm", "--needed", "base-devel", "git"];
    let (ok, _) = run_command_with_output_detailed("sudo", &deps, "pacman", true);
    if !ok {
        println!("{}", "Failed to install build dependencies".red());
        return None;
    }

    let tmpdir = "/tmp/yay-build";
    let _ = std::fs::remove_dir_all(tmpdir);

    println!("{}", "Cloning yay from AUR...".yellow());
    let (ok, _) = run_command_with_output_detailed("git", &["clone", "https://aur.archlinux.org/yay.git", tmpdir], "git", true);
    if !ok {
        println!("{}", "Failed to clone yay".red());
        return None;
    }

    println!("{}", "Building and installing yay...".yellow());
    let status = std::process::Command::new("makepkg")
        .args(&["-si", "--noconfirm"])
        .current_dir(tmpdir)
        .status();
    if !status.is_ok() || !status.unwrap().success() {
        println!("{}", "Failed to build yay".red());
        return None;
    }

    let _ = std::fs::remove_dir_all(tmpdir);
    detect_aur_helper()
}

fn handle_search(system: System, flags: Flags, packages: &[&str]) {
    if packages.is_empty() {
        println!("{}", "No package given".red());
        exit(1);
    }
    let package = packages[0];

    if flags.flatpak {
        if system == System::Linux {
            if !ensure_flatpak_installed() { return; }
            match search_package_flatpak(package) {
                Some(result) => println!("{}", format_search_box(package, &result).bright_magenta()),
                None => println!("{}", format!("{}: Package not found", package).red()),
            }
        } else {
            println!("{}", "Flatpak search only available on Linux".red());
        }
        return;
    }

    match system {
        System::Linux => match detect_linux_package_manager() {
            Some(manager) => match search_package_linux(package, &manager) {
                Some(result) => println!("{}", format_search_box(package, &result).bright_blue()),
                None => {
                    if manager.program == "pacman" {
                        if let Some(aur) = detect_aur_helper() {
                            if let Some(result) = search_package_linux(package, &aur) {
                                println!("{}", format_search_box(package, &result).truecolor(255, 165, 0));
                                return;
                            }
                        }
                    }
                    println!("{}", format!("{}: Package not found", package).red());
                }
            },
            None => println!("{}", "No supported package manager found".red()),
        },
        System::MacOS => match detect_macos_package_manager() {
            Some(manager) => match search_package_linux(package, &manager) {
                Some(result) => println!("{}", format_search_box(package, &result).bright_blue()),
                None => println!("{}", format!("{}: Package not found", package).red()),
            },
            None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
        },
        System::Windows => println!("{}", "Search not yet supported for Windows".yellow()),
        System::Unknown => println!("{}", "Unsupported system".red()),
    }
}

fn handle_list(system: System, flags: Flags) {
    match system {
        System::Linux => {
            if flags.flatpak {
                if !ensure_flatpak_installed() { return; }
                run_command_with_output_detailed("flatpak", &["list", "--app"], "flatpak", true);
            } else {
                match detect_linux_package_manager() {
                    Some(manager) => {
                        run_command_with_output_detailed(manager.program, manager.list_args, manager.program, true);
                    }
                    None => println!("{}", "No supported package manager found".red()),
                }
            }
        }
        System::MacOS => {
            if flags.flatpak {
                println!("{}", "Flatpak is not available on macOS".red());
            } else {
                match detect_macos_package_manager() {
                    Some(manager) => {
                        run_command_with_output_detailed(manager.program, manager.list_args, manager.program, true);
                    }
                    None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
                }
            }
        }
        System::Windows => println!("{}", "List not yet supported for Windows".yellow()),
        System::Unknown => println!("{}", "Unsupported system".red()),
    }
}

fn handle_update(system: System, flags: Flags, packages: &[&str]) {
    let is_quiet = flags.quiet;
    let skip_confirm = flags.autoinstall;

    if flags.flatpak {
        if system != System::Linux {
            println!("{}", "Flatpak is not available on this system".red());
            return;
        }
        if !ensure_flatpak_installed() { return; }

        if packages.is_empty() {
            println!("{}", format_box_multiple("Update Flatpak", vec![
                ("All installed flatpaks".to_string(), String::new(), String::new())
            ]).bright_magenta());
            if !skip_confirm && !ask_update_confirmation() {
                println!("{}", "Update cancelled".yellow());
                return;
            }
            if flags.dry_run {
                println!("Would update all flatpaks");
                return;
            }
            let (status, _) = run_command_with_output_detailed("flatpak", &["update", "-y"], "flatpak", !is_quiet);
            print_result(Action::Update, status);
        } else {
            let packages_info: Vec<(String, String, String)> = packages.iter()
                .map(|p| (p.to_string(), String::new(), String::new()))
                .collect();
            println!("{}", format_box_multiple("Update Flatpak", packages_info).bright_magenta());

            if !skip_confirm && !ask_update_confirmation() {
                println!("{}", "Update cancelled".yellow());
                return;
            }
            if flags.dry_run {
                for package in packages {
                    println!("Would update {} via flatpak", package);
                }
                return;
            }
            let mut args: Vec<&str> = vec!["update", "-y"];
            for package in packages {
                args.push(package);
            }
            let (status, _) = run_command_with_output_detailed("flatpak", &args, "flatpak", !is_quiet);
            print_result(Action::Update, status);
        }
        return;
    }

    match system {
        System::Linux => match detect_linux_package_manager() {
            Some(manager) => {
                if packages.is_empty() {
                    println!("{}", format_box_multiple("Update", vec![
                        ("All packages".to_string(), String::new(), String::new())
                    ]).bright_blue());
                    if !skip_confirm && !ask_update_confirmation() {
                        println!("{}", "Update cancelled".yellow());
                        return;
                    }

                    if !manager.update_cache_args.is_empty() {
                        let mut cache_args = vec![manager.program];
                        cache_args.extend(manager.update_cache_args);
                        let (cache_prog, cache_slice) = if requires_sudo(&manager) { ("sudo", cache_args.as_slice()) } else { (manager.program, &cache_args[1..]) };
                        let _ = run_command_with_output_detailed(cache_prog, cache_slice, manager.program, !is_quiet);
                    }

                    if flags.dry_run {
                        println!("Would upgrade all packages via {}", manager.program);
                        return;
                    }

                    let mut args = vec![manager.program];
                    if flags.dry_run {
                        args.extend(manager.dry_run_args);
                    }
                    args.extend(manager.update_args);
                    let (prog, cmd_args) = if requires_sudo(&manager) { ("sudo", args.as_slice()) } else { (manager.program, &args[1..]) };
                    let (status, _) = run_command_with_output_detailed(prog, cmd_args, manager.program, !is_quiet);
                    print_result(Action::Update, status);
                } else {
                    let packages_info: Vec<(String, String, String)> = packages.iter()
                        .map(|p| (p.to_string(), String::new(), String::new()))
                        .collect();
                    println!("{}", format_box_multiple("Update", packages_info).bright_blue());

                    if !skip_confirm && !ask_update_confirmation() {
                        println!("{}", "Update cancelled".yellow());
                        return;
                    }

                    if flags.dry_run {
                        for package in packages {
                            println!("Would update {} via {}", package, manager.program);
                        }
                        return;
                    }

                    let mut args = vec![manager.program];
                    if flags.dry_run {
                        args.extend(manager.dry_run_args);
                    }
                    args.extend(manager.update_single_args);
                    for package in packages {
                        args.push(package);
                    }
                    let (prog, cmd_args) = if requires_sudo(&manager) { ("sudo", args.as_slice()) } else { (manager.program, &args[1..]) };
                    let (status, _) = run_command_with_output_detailed(prog, cmd_args, manager.program, !is_quiet);
                    print_result(Action::Update, status);
                }
            }
            None => println!("{}", "No supported package manager found".red()),
        },
        System::MacOS => match detect_macos_package_manager() {
            Some(manager) => {
                if packages.is_empty() {
                    println!("{}", format_box_multiple("Update", vec![
                        ("All brew packages".to_string(), String::new(), String::new())
                    ]).bright_blue());
                    if !skip_confirm && !ask_update_confirmation() {
                        println!("{}", "Update cancelled".yellow());
                        return;
                    }

                    if flags.dry_run {
                        println!("Would run brew update and upgrade all packages");
                        return;
                    }

                    let mut update_args = vec!["update"];
                    if flags.dry_run {
                        update_args.extend(manager.dry_run_args);
                    }
                    let _ = run_command_with_output_detailed(manager.program, &update_args, manager.program, !is_quiet);

                    let mut upgrade_args = manager.update_args.to_vec();
                    if flags.dry_run {
                        upgrade_args.extend(manager.dry_run_args);
                    }
                    let (status, _) = run_command_with_output_detailed(manager.program, &upgrade_args, manager.program, !is_quiet);
                    print_result(Action::Update, status);
                } else {
                    let packages_info: Vec<(String, String, String)> = packages.iter()
                        .map(|p| (p.to_string(), "homebrew".to_string(), String::new()))
                        .collect();
                    println!("{}", format_box_multiple("Update", packages_info).bright_blue());

                    if !skip_confirm && !ask_update_confirmation() {
                        println!("{}", "Update cancelled".yellow());
                        return;
                    }

                    if flags.dry_run {
                        for package in packages {
                            println!("Would upgrade {} via brew", package);
                        }
                        return;
                    }

                    let mut args = manager.update_single_args.to_vec();
                    if flags.dry_run {
                        args.extend(manager.dry_run_args);
                    }
                    for package in packages {
                        args.push(package);
                    }
                    let (status, _) = run_command_with_output_detailed(manager.program, &args, manager.program, !is_quiet);
                    print_result(Action::Update, status);
                }
            }
            None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
        },
        System::Windows => println!("{}", "Update not yet supported for Windows".yellow()),
        System::Unknown => println!("{}", "Unsupported system".red()),
    }
}

fn handle_install(system: System, flags: Flags, packages: &[&str]) {
    if packages.is_empty() {
        println!("{}", "No package given".red());
        exit(1);
    }

    let is_quiet = flags.quiet;
    let skip_confirm = flags.autoinstall;

    if flags.flatpak {
        if system != System::Linux {
            println!("{}", "Flatpak is not available on this system".red());
            return;
        }
        if !ensure_flatpak_installed() { return; }

        let mut all_valid = true;
        let mut packages_info = Vec::new();
        let mut full_app_ids = Vec::new();

        for package in packages {
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

        if flags.dry_run {
            for full_app_id in &full_app_ids {
                println!("Would install {} via flatpak", full_app_id);
            }
            return;
        }

        for full_app_id in full_app_ids {
            let (status, _) = run_command_with_output_detailed("flatpak", &["install", "-y", "flathub", &full_app_id], "flatpak", !is_quiet);
            print_result(Action::Install, status);
        }
        return;
    }

    match system {
        System::Linux => {
            let manager = match detect_linux_package_manager() {
                Some(m) => m,
                None => { println!("{}", "No supported package manager found".red()); return; }
            };

            let mut effective = manager;
            if effective.program == "pacman" {
                let mut needs_aur = false;
                let mut bogus = false;
                for package in packages {
                    let (repo, _) = search_info(&effective, package);
                    if repo.is_empty() {
                        if let Some(aur) = detect_aur_helper() {
                            let (a_repo, _) = search_info(&aur, package);
                            if a_repo.is_empty() {
                                println!("{}", format!("{}: Package not found in any repository", package).red());
                                bogus = true;
                            } else {
                                needs_aur = true;
                            }
                        } else {
                            needs_aur = true;
                        }
                    }
                }
                if bogus { return; }
                if needs_aur {
                    match ensure_aur_helper() {
                        Some(aur) => effective = aur,
                        None => return,
                    }
                }
            }

            let mut packages_info = Vec::new();
            for package in packages {
                let (repo, size) = search_info(&effective, package);
                packages_info.push((package.to_string(), if repo.is_empty() { "AUR".to_string() } else { repo }, size));
            }

            let box_str = format_box_multiple("Install", packages_info);
            if effective.program == "yay" || effective.program == "paru" {
                println!("{}", box_str.truecolor(255, 165, 0));
            } else {
                println!("{}", box_str.bright_blue());
            }

            if !skip_confirm && !ask_confirmation() {
                println!("{}", "Installation cancelled".yellow());
                return;
            }

            if flags.dry_run {
                for package in packages {
                    println!("Would install {} via {}", package, effective.program);
                }
                return;
            }

            let mut args = vec![effective.program];
            if flags.dry_run {
                args.extend(effective.dry_run_args);
            }
            args.extend(effective.install_args);
            for package in packages {
                args.push(package);
            }
            let (prog, cmd_args) = if requires_sudo(&effective) { ("sudo", args.as_slice()) } else { (effective.program, &args[1..]) };
            let (status, _) = run_command_with_output_detailed(prog, cmd_args, effective.program, !is_quiet);
            print_result(Action::Install, status);
        }
        System::MacOS => match detect_macos_package_manager() {
            Some(manager) => {
                let mut packages_info = Vec::new();

                for package in packages {
                    let (repo, size) = search_info(&manager, package);
                    if size.is_empty() {
                        eprintln!("{}", format!("Warning: {} not found in repositories, attempting install anyway", package).yellow());
                    }
                    packages_info.push((package.to_string(), repo, size));
                }

                println!("{}", format_box_multiple("Install", packages_info).bright_blue());

                if !skip_confirm && !ask_confirmation() {
                    println!("{}", "Installation cancelled".yellow());
                    return;
                }

                if flags.dry_run {
                    for package in packages {
                        println!("Would install {} via brew", package);
                    }
                    return;
                }

                let mut args = manager.install_args.to_vec();
                if flags.dry_run {
                    args.extend(manager.dry_run_args);
                }
                for package in packages {
                    args.push(package);
                }
                let (status, _) = run_command_with_output_detailed(manager.program, &args, manager.program, !is_quiet);
                print_result(Action::Install, status);
            }
            None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
        },
        System::Windows => {
            let winget = PackageManager {
                program: "winget",
                install_args: &["install", "--exact"],
                remove_args: &["uninstall", "--exact"],
                update_args: &["upgrade"],
                update_single_args: &["upgrade", "--exact"],
                list_args: &["list"],
                search_args: &["search"],
                dry_run_args: &["--dry-run"],
                update_cache_args: &[],
            };
            if flags.dry_run {
                for package in packages {
                    println!("Would install {} via winget", package);
                }
                return;
            }
            let mut args = winget.install_args.to_vec();
            if flags.dry_run {
                args.extend(winget.dry_run_args);
            }
            for package in packages {
                args.push(package);
            }
            let (status, _) = run_command_with_output_detailed(winget.program, &args, winget.program, !is_quiet);
            print_result(Action::Install, status);
        }
        System::Unknown => println!("{}", "Unsupported system".red()),
    }
}

fn handle_remove(system: System, flags: Flags, packages: &[&str]) {
    if packages.is_empty() {
        println!("{}", "No package given".red());
        exit(1);
    }

    let is_quiet = flags.quiet;
    let skip_confirm = flags.autoinstall;

    if flags.flatpak {
        if system != System::Linux {
            println!("{}", "Flatpak is not available on this system".red());
            return;
        }
        if !ensure_flatpak_installed() { return; }

        let mut all_valid = true;
        let mut packages_info = Vec::new();
        let mut app_ids = Vec::new();

        for package in packages {
            let mut app_id = package.to_string();

            if !package.contains(".") {
                if let Some(id) = fuzzy_match_flatpak(package) {
                    app_id = id;
                }
            }

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

        if flags.dry_run {
            for app_id in &app_ids {
                println!("Would uninstall {} via flatpak", app_id);
            }
            return;
        }

        for app_id in app_ids {
            let (status, _) = run_command_with_output_detailed("flatpak", &["uninstall", "-y", &app_id], "flatpak", !is_quiet);
            print_result(Action::Remove, status);
        }
        return;
    }

    match system {
        System::Linux => {
            let manager = match detect_linux_package_manager() {
                Some(m) => m,
                None => { println!("{}", "No supported package manager found".red()); return; }
            };

            let mut effective = manager;
            if effective.program == "pacman" {
                let mut needs_aur = false;
                let mut bogus = false;
                for package in packages {
                    if get_installed_package_info(&effective, package).1.is_empty() {
                        if let Some(aur) = detect_aur_helper() {
                            if get_installed_package_info(&aur, package).1.is_empty() {
                                println!("{}", format!("{}: Package not installed or doesn't exist", package).red());
                                bogus = true;
                            } else {
                                needs_aur = true;
                            }
                        } else {
                            needs_aur = true;
                        }
                    }
                }
                if bogus { return; }
                if needs_aur {
                    match ensure_aur_helper() {
                        Some(aur) => effective = aur,
                        None => return,
                    }
                }
            }

            let mut packages_info = Vec::new();
            for package in packages {
                let (_, size) = get_installed_package_info(&effective, package);
                packages_info.push((package.to_string(), String::new(), size));
            }

            println!("{}", format_box_multiple("Remove", packages_info).bright_red());

            if !skip_confirm && !ask_removal_confirmation() {
                println!("{}", "Removal cancelled".yellow());
                return;
            }

            if flags.dry_run {
                for package in packages {
                    println!("Would remove {} via {}", package, effective.program);
                }
                return;
            }

            let mut args = vec![effective.program];
            if flags.dry_run {
                args.extend(effective.dry_run_args);
            }
            args.extend(effective.remove_args);
            for package in packages {
                args.push(package);
            }
            let (prog, cmd_args) = if requires_sudo(&effective) { ("sudo", args.as_slice()) } else { (effective.program, &args[1..]) };
            let (status, _) = run_command_with_output_detailed(prog, cmd_args, effective.program, !is_quiet);
            print_result(Action::Remove, status);
        }
        System::MacOS => match detect_macos_package_manager() {
            Some(manager) => {
                let mut packages_info = Vec::new();

                for package in packages {
                    let (_, size) = get_installed_package_info(&manager, package);
                    if size.is_empty() {
                        eprintln!("{}", format!("Warning: {} not detected as installed, attempting removal anyway", package).yellow());
                    }
                    packages_info.push((package.to_string(), String::new(), size));
                }

                println!("{}", format_box_multiple("Remove", packages_info).bright_red());

                if !skip_confirm && !ask_removal_confirmation() {
                    println!("{}", "Removal cancelled".yellow());
                    return;
                }

                if flags.dry_run {
                    for package in packages {
                        println!("Would uninstall {} via brew", package);
                    }
                    return;
                }

                let mut args = manager.remove_args.to_vec();
                if flags.dry_run {
                    args.extend(manager.dry_run_args);
                }
                for package in packages {
                    args.push(package);
                }
                let (status, _) = run_command_with_output_detailed(manager.program, &args, manager.program, !is_quiet);
                print_result(Action::Remove, status);
            }
            None => println!("{}", "No package manager found (is Homebrew installed?)".red()),
        },
        System::Windows => {
            let winget = PackageManager {
                program: "winget",
                install_args: &["install", "--exact"],
                remove_args: &["uninstall", "--exact"],
                update_args: &["upgrade"],
                update_single_args: &["upgrade", "--exact"],
                list_args: &["list"],
                search_args: &["search"],
                dry_run_args: &["--dry-run"],
                update_cache_args: &[],
            };
            if flags.dry_run {
                for package in packages {
                    println!("Would uninstall {} via winget", package);
                }
                return;
            }
            let mut args = winget.remove_args.to_vec();
            if flags.dry_run {
                args.extend(winget.dry_run_args);
            }
            for package in packages {
                args.push(package);
            }
            let (status, _) = run_command_with_output_detailed(winget.program, &args, winget.program, !is_quiet);
            print_result(Action::Remove, status);
        }
        System::Unknown => println!("{}", "Unsupported system".red()),
    }
}
