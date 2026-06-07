use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum System {
    Windows,
    Linux,
    MacOS,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PackageManager {
    pub program: &'static str,
    pub install_args: &'static [&'static str],
    pub remove_args: &'static [&'static str],
    pub update_args: &'static [&'static str],
    pub update_single_args: &'static [&'static str],
    pub list_args: &'static [&'static str],
    pub search_args: &'static [&'static str],
}

pub fn detect_system() -> System {
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

pub fn command_exists(program: &str) -> bool {
    let checker = if cfg!(target_os = "windows") { "where" } else { "which" };
    match Command::new(checker)
        .arg(program)
        .output() {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

pub fn detect_linux_package_manager() -> Option<PackageManager> {
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

pub fn detect_macos_package_manager() -> Option<PackageManager> {
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
