mod action;
mod backend;
mod runner;
mod search;
mod ui;

use std::env;
use std::process::exit;
use colored::*;

use action::{Action, Flags, parse_action};
use backend::{System, detect_system, detect_linux_package_manager, detect_macos_package_manager, PackageManager};
use runner::run_command_with_output_detailed;
use search::{search_info, search_package_linux, search_package_flatpak, get_installed_package_info, fuzzy_match_flatpak, fuzzy_match_flatpak_with_size, is_flatpak_installed};
use ui::{format_box_multiple, format_search_box, print_result, ask_confirmation, ask_removal_confirmation, ask_update_confirmation};

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

    let system = detect_system();
    let filtered: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

    let (action, flags) = parse_action(filtered.get(0).unwrap_or(&""))
        .unwrap_or_else(|| {
            println!("{}", "Invalid command".red());
            exit(1);
        });

    let packages: Vec<&str> = filtered.get(1..).unwrap_or(&[]).to_vec();

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
    println!("{}", "Usage: hibrid [-I|-R|-U|-L][a][q][f][d] [pkg]".bright_white().bold());
    println!("       hibrid -V");
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
    println!("  {} Dry run (preview without making changes)", "d".bright_cyan());
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
    println!("  hibrid {} vim", "-Id".green());
    println!("  hibrid {} package", "-R".red());
    println!("  hibrid {} spotify", "-If".bright_magenta());
    println!("  hibrid {}", "-U".yellow());
    println!("  hibrid {} vim", "-U".yellow());
    println!("  hibrid {}", "-L".cyan());
    println!("  hibrid {}", "-V".bright_white());
}

fn handle_search(system: System, flags: Flags, packages: &[&str]) {
    if packages.is_empty() {
        println!("{}", "No package given".red());
        exit(1);
    }
    let package = packages[0];

    if flags.flatpak {
        if system == System::Linux {
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
                Some(result) => println!("{}", format_search_box(package, &result).bright_cyan()),
                None => println!("{}", format!("{}: Package not found", package).red()),
            },
            None => println!("{}", "No supported package manager found".red()),
        },
        System::MacOS => match detect_macos_package_manager() {
            Some(manager) => match search_package_linux(package, &manager) {
                Some(result) => println!("{}", format_search_box(package, &result).bright_cyan()),
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
                run_command_with_output_detailed("flatpak", &["list", "--app"], "flatpak", true);
            } else {
                match detect_linux_package_manager() {
                    Some(manager) => {
                        let mut args = vec![manager.program];
                        args.extend(manager.list_args);
                        run_command_with_output_detailed("sudo", &args, manager.program, true);
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
                    ]).bright_cyan());
                    if !skip_confirm && !ask_update_confirmation() {
                        println!("{}", "Update cancelled".yellow());
                        return;
                    }

                    if !manager.update_cache_args.is_empty() {
                        let mut cache_args = vec![manager.program];
                        cache_args.extend(manager.update_cache_args);
                        let _ = run_command_with_output_detailed("sudo", &cache_args, manager.program, !is_quiet);
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
                    let (status, _) = run_command_with_output_detailed("sudo", &args, manager.program, !is_quiet);
                    print_result(Action::Update, status);
                } else {
                    let packages_info: Vec<(String, String, String)> = packages.iter()
                        .map(|p| (p.to_string(), String::new(), String::new()))
                        .collect();
                    println!("{}", format_box_multiple("Update", packages_info).bright_cyan());

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
                    let (status, _) = run_command_with_output_detailed("sudo", &args, manager.program, !is_quiet);
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
                    ]).bright_cyan());
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
                    println!("{}", format_box_multiple("Update", packages_info).bright_cyan());

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
        System::Linux => match detect_linux_package_manager() {
            Some(manager) => {
                let mut all_valid = true;
                let mut packages_info = Vec::new();

                for package in packages {
                    let (repo, size) = search_info(&manager, package);
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

                if flags.dry_run {
                    for package in packages {
                        println!("Would install {} via {}", package, manager.program);
                    }
                    return;
                }

                let mut args = vec![manager.program];
                if flags.dry_run {
                    args.extend(manager.dry_run_args);
                }
                args.extend(manager.install_args);
                for package in packages {
                    args.push(package);
                }
                let (status, _) = run_command_with_output_detailed("sudo", &args, manager.program, !is_quiet);
                print_result(Action::Install, status);
            }
            None => println!("{}", "No supported package manager found".red()),
        },
        System::MacOS => match detect_macos_package_manager() {
            Some(manager) => {
                let mut all_valid = true;
                let mut packages_info = Vec::new();

                for package in packages {
                    let (repo, size) = search_info(&manager, package);
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
        System::Linux => match detect_linux_package_manager() {
            Some(manager) => {
                let mut all_valid = true;
                let mut packages_info = Vec::new();

                for package in packages {
                    let (_, size) = get_installed_package_info(&manager, package);
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

                if flags.dry_run {
                    for package in packages {
                        println!("Would remove {} via {}", package, manager.program);
                    }
                    return;
                }

                let mut args = vec![manager.program];
                if flags.dry_run {
                    args.extend(manager.dry_run_args);
                }
                args.extend(manager.remove_args);
                for package in packages {
                    args.push(package);
                }
                let (status, _) = run_command_with_output_detailed("sudo", &args, manager.program, !is_quiet);
                print_result(Action::Remove, status);
            }
            None => println!("{}", "No supported package manager found".red()),
        },
        System::MacOS => match detect_macos_package_manager() {
            Some(manager) => {
                let mut all_valid = true;
                let mut packages_info = Vec::new();

                for package in packages {
                    let (_, size) = get_installed_package_info(&manager, package);
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
