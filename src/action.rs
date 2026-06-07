#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Install,
    Remove,
    Update,
    List,
    Search,
}

#[derive(Debug, Clone, Copy)]
pub struct Flags {
    pub autoinstall: bool,
    pub quiet: bool,
    pub flatpak: bool,
    pub dry_run: bool,
}

pub fn parse_action(flag: &str) -> Option<(Action, Flags)> {
    if !flag.starts_with('-') || flag.len() < 2 {
        return None;
    }

    let flag_chars = &flag[1..];
    let base = flag_chars.chars().next()?;
    let modifiers = &flag_chars[1..];

    let valid_mods = |s: &str, allowed: &[char]| s.chars().all(|c| allowed.contains(&c));

    match base {
        'I' | 'R' | 'U' => {
            if !valid_mods(modifiers, &['a', 'q', 'f', 'd']) {
                return None;
            }
            let action = match base {
                'I' => Action::Install,
                'R' => Action::Remove,
                _ => Action::Update,
            };
            Some((action, Flags {
                autoinstall: modifiers.contains('a'),
                quiet: modifiers.contains('q'),
                flatpak: modifiers.contains('f'),
                dry_run: modifiers.contains('d'),
            }))
        }
        'L' | 'S' => {
            if !valid_mods(modifiers, &['f']) {
                return None;
            }
            let action = match base {
                'L' => Action::List,
                _ => Action::Search,
            };
            Some((action, Flags {
                autoinstall: false,
                quiet: false,
                flatpak: modifiers.contains('f'),
                dry_run: false,
            }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_install_basic() {
        let (action, flags) = parse_action("-I").unwrap();
        assert_eq!(action, Action::Install);
        assert!(!flags.autoinstall && !flags.quiet && !flags.flatpak && !flags.dry_run);
    }

    #[test]
    fn parse_install_auto() {
        let (action, flags) = parse_action("-Ia").unwrap();
        assert_eq!(action, Action::Install);
        assert!(flags.autoinstall);
    }

    #[test]
    fn parse_install_quiet() {
        let (action, flags) = parse_action("-Iq").unwrap();
        assert_eq!(action, Action::Install);
        assert!(flags.quiet);
    }

    #[test]
    fn parse_install_flatpak() {
        let (action, flags) = parse_action("-If").unwrap();
        assert_eq!(action, Action::Install);
        assert!(flags.flatpak);
    }

    #[test]
    fn parse_install_dry_run() {
        let (action, flags) = parse_action("-Id").unwrap();
        assert_eq!(action, Action::Install);
        assert!(flags.dry_run);
    }

    #[test]
    fn parse_install_all_mods() {
        let (action, flags) = parse_action("-Iaqfd").unwrap();
        assert_eq!(action, Action::Install);
        assert!(flags.autoinstall && flags.quiet && flags.flatpak && flags.dry_run);
    }

    #[test]
    fn parse_remove_basic() {
        let (action, _flags) = parse_action("-R").unwrap();
        assert_eq!(action, Action::Remove);
    }

    #[test]
    fn parse_remove_dry_run() {
        let (action, flags) = parse_action("-Rd").unwrap();
        assert_eq!(action, Action::Remove);
        assert!(flags.dry_run);
    }

    #[test]
    fn parse_update_basic() {
        let (action, _flags) = parse_action("-U").unwrap();
        assert_eq!(action, Action::Update);
    }

    #[test]
    fn parse_update_dry_run() {
        let (action, flags) = parse_action("-Ud").unwrap();
        assert_eq!(action, Action::Update);
        assert!(flags.dry_run);
    }

    #[test]
    fn parse_list() {
        let (action, _flags) = parse_action("-L").unwrap();
        assert_eq!(action, Action::List);
    }

    #[test]
    fn parse_search() {
        let (action, _flags) = parse_action("-S").unwrap();
        assert_eq!(action, Action::Search);
    }

    #[test]
    fn parse_invalid_no_dash() {
        assert!(parse_action("I").is_none());
    }

    #[test]
    fn parse_invalid_base() {
        assert!(parse_action("-X").is_none());
    }

    #[test]
    fn parse_invalid_modifier() {
        assert!(parse_action("-Ix").is_none());
    }

    #[test]
    fn parse_invalid_modifier_on_list() {
        assert!(parse_action("-Ld").is_none());
    }

    #[test]
    fn parse_empty() {
        assert!(parse_action("").is_none());
    }
}
