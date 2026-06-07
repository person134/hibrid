use colored::*;
use std::io::{self, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use std::time::Duration;

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

#[allow(dead_code)]
pub fn run_command_with_output(program: &str, args: &[&str], pkg_manager: &str) -> (bool, String) {
    run_command_with_output_detailed(program, args, pkg_manager, false)
}

pub fn run_command_with_output_detailed(program: &str, args: &[&str], pkg_manager: &str, detailed: bool) -> (bool, String) {
    if detailed {
        let mut cmd = Command::new(program);
        cmd.args(args);
        if program == "sudo" {
            cmd.stdin(Stdio::inherit());
        }
        match cmd.status() {
            Ok(status) => (status.success(), String::new()),
            Err(e) => {
                eprintln!("{}", format!("Failed to execute command: {}", e).red());
                (false, String::new())
            }
        }
    } else {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        let spinner_thread = thread::spawn(move || show_spinner(running_clone));

        let mut cmd = Command::new(program);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        if program == "sudo" {
            cmd.stdin(Stdio::inherit());
        }

        let result = match cmd.output() {
            Ok(output) => {
                let success = output.status.success();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let info = extract_package_info(&stdout, pkg_manager);

                if !success {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let trimmed = stderr.trim();
                    if !trimmed.is_empty() {
                        eprintln!("{}", trimmed.red());
                    }
                }

                (success, info)
            }
            Err(e) => {
                eprintln!("{}", format!("Failed to execute command: {}", e).red());
                (false, String::new())
            }
        };

        running.store(false, Ordering::Relaxed);
        let _ = spinner_thread.join();
        result
    }
}
