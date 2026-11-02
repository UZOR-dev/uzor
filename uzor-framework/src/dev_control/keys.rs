//! Key names for the `key` command: `Enter`, `Tab`, `Ctrl+A`, `Shift+Tab`,
//! `F5`, `ArrowDown`, … (case-insensitive, `+`-joined modifiers).

use uzor::input::{KeyCode, ModifierKeys};

/// A parsed key chord.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chord {
    /// The key.
    pub code: KeyCode,
    /// Held modifiers.
    pub mods: ModifierKeys,
    /// The text a plain press types (`None` for named keys and shortcuts).
    pub text: Option<String>,
}

/// Parse a chord such as `Ctrl+Shift+Z`.
pub fn parse(spec: &str) -> Result<Chord, String> {
    let parts: Vec<&str> = spec.split('+').map(str::trim).collect();
    if parts.iter().any(|p| p.is_empty()) {
        // "Ctrl++" means Ctrl and the plus key.
        if spec.ends_with("++") || spec == "+" {
            let head = spec.trim_end_matches('+');
            let mut c = if head.is_empty() { Chord { code: KeyCode::Plus, mods: ModifierKeys::default(), text: None } } else { parse(&format!("{head}+Plus"))? };
            if c.mods == ModifierKeys::default() {
                c.text = Some("+".into());
            }
            return Ok(c);
        }
        return Err(format!("bad key '{spec}'"));
    }
    let (last, mods_part) = parts.split_last().ok_or_else(|| format!("bad key '{spec}'"))?;
    let mut mods = ModifierKeys::default();
    for m in mods_part {
        match m.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => mods.ctrl = true,
            "shift" => mods.shift = true,
            "alt" | "option" => mods.alt = true,
            "meta" | "cmd" | "super" | "win" => mods.meta = true,
            other => return Err(format!("unknown modifier '{other}'")),
        }
    }
    let (code, plain) = code_of(last).ok_or_else(|| format!("unknown key '{last}'"))?;
    let text = if mods.ctrl || mods.alt || mods.meta {
        None
    } else {
        plain.map(|t| if mods.shift { t.to_uppercase() } else { t })
    };
    Ok(Chord { code, mods, text })
}

/// The key code for a name, and the text a plain press of it types.
fn code_of(name: &str) -> Option<(KeyCode, Option<String>)> {
    use KeyCode as K;
    let lower = name.to_ascii_lowercase();
    if name.chars().count() == 1 {
        let c = name.chars().next()?;
        let t = Some(c.to_string());
        if c.is_ascii_alphabetic() {
            const L: [KeyCode; 26] = [
                K::A, K::B, K::C, K::D, K::E, K::F, K::G, K::H, K::I, K::J, K::K, K::L, K::M,
                K::N, K::O, K::P, K::Q, K::R, K::S, K::T, K::U, K::V, K::W, K::X, K::Y, K::Z,
            ];
            return Some((L[(c.to_ascii_lowercase() as u8 - b'a') as usize], Some(c.to_ascii_lowercase().to_string())));
        }
        if c.is_ascii_digit() {
            const D: [KeyCode; 10] = [K::Num0, K::Num1, K::Num2, K::Num3, K::Num4, K::Num5, K::Num6, K::Num7, K::Num8, K::Num9];
            return Some((D[(c as u8 - b'0') as usize], t));
        }
        return Some(match c {
            '-' => (K::Minus, t),
            '[' => (K::BracketLeft, t),
            ']' => (K::BracketRight, t),
            ' ' => (K::Space, t),
            _ => (K::Unknown, t),
        });
    }
    let named = match lower.as_str() {
        "enter" | "return" => K::Enter,
        "tab" => K::Tab,
        "esc" | "escape" => K::Escape,
        "backspace" => K::Backspace,
        "delete" | "del" => K::Delete,
        "insert" | "ins" => K::Insert,
        "home" => K::Home,
        "end" => K::End,
        "pageup" | "pgup" => K::PageUp,
        "pagedown" | "pgdn" => K::PageDown,
        "up" | "arrowup" => K::ArrowUp,
        "down" | "arrowdown" => K::ArrowDown,
        "left" | "arrowleft" => K::ArrowLeft,
        "right" | "arrowright" => K::ArrowRight,
        "space" => return Some((K::Space, Some(" ".into()))),
        "plus" => return Some((K::Plus, Some("+".into()))),
        "minus" => return Some((K::Minus, Some("-".into()))),
        _ => {
            let n: u8 = lower.strip_prefix('f')?.parse().ok()?;
            const F: [KeyCode; 12] = [K::F1, K::F2, K::F3, K::F4, K::F5, K::F6, K::F7, K::F8, K::F9, K::F10, K::F11, K::F12];
            *F.get(usize::from(n).checked_sub(1)?)?
        }
    };
    Some((named, None))
}

/// The key code a typed character maps to (`Unknown` for most text; the
/// text itself rides along, as a real keyboard's does).
pub fn code_for_char(c: char) -> KeyCode {
    match c {
        '\n' | '\r' => KeyCode::Enter,
        '\t' => KeyCode::Tab,
        _ => code_of(&c.to_string()).map_or(KeyCode::Unknown, |(k, _)| k),
    }
}

/// The chord name of a key press (the inverse of [`parse`]): `Enter`,
/// `Ctrl+A`, `Shift+Tab`, `F5`… `None` for keys without a name.
pub fn chord_name(code: KeyCode, mods: ModifierKeys) -> Option<String> {
    use KeyCode as K;
    const L: [(KeyCode, &str); 26] = [
        (K::A, "A"), (K::B, "B"), (K::C, "C"), (K::D, "D"), (K::E, "E"), (K::F, "F"), (K::G, "G"),
        (K::H, "H"), (K::I, "I"), (K::J, "J"), (K::K, "K"), (K::L, "L"), (K::M, "M"), (K::N, "N"),
        (K::O, "O"), (K::P, "P"), (K::Q, "Q"), (K::R, "R"), (K::S, "S"), (K::T, "T"), (K::U, "U"),
        (K::V, "V"), (K::W, "W"), (K::X, "X"), (K::Y, "Y"), (K::Z, "Z"),
    ];
    const N: [(KeyCode, &str); 41] = [
        (K::Num0, "0"), (K::Num1, "1"), (K::Num2, "2"), (K::Num3, "3"), (K::Num4, "4"),
        (K::Num5, "5"), (K::Num6, "6"), (K::Num7, "7"), (K::Num8, "8"), (K::Num9, "9"),
        (K::Enter, "Enter"), (K::Tab, "Tab"), (K::Escape, "Escape"), (K::Backspace, "Backspace"),
        (K::Delete, "Delete"), (K::Insert, "Insert"), (K::Home, "Home"), (K::End, "End"),
        (K::PageUp, "PageUp"), (K::PageDown, "PageDown"), (K::ArrowUp, "ArrowUp"),
        (K::ArrowDown, "ArrowDown"), (K::ArrowLeft, "ArrowLeft"), (K::ArrowRight, "ArrowRight"),
        (K::Space, "Space"), (K::Plus, "Plus"), (K::Minus, "Minus"),
        (K::BracketLeft, "["), (K::BracketRight, "]"),
        (K::F1, "F1"), (K::F2, "F2"), (K::F3, "F3"), (K::F4, "F4"), (K::F5, "F5"), (K::F6, "F6"),
        (K::F7, "F7"), (K::F8, "F8"), (K::F9, "F9"), (K::F10, "F10"), (K::F11, "F11"), (K::F12, "F12"),
    ];
    let name = L.iter().chain(N.iter()).find(|(k, _)| *k == code).map(|(_, n)| *n)?;
    let mut out = String::new();
    for (on, m) in [(mods.ctrl, "Ctrl+"), (mods.alt, "Alt+"), (mods.shift, "Shift+"), (mods.meta, "Meta+")] {
        if on {
            out.push_str(m);
        }
    }
    out.push_str(name);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chords() {
        let c = parse("Ctrl+A").unwrap();
        assert_eq!(c.code, KeyCode::A);
        assert!(c.mods.ctrl);
        assert_eq!(c.text, None);
        assert_eq!(parse("enter").unwrap().code, KeyCode::Enter);
        assert_eq!(parse("Shift+Tab").unwrap().mods.shift, true);
        assert_eq!(parse("F12").unwrap().code, KeyCode::F12);
        assert_eq!(parse("a").unwrap().text.as_deref(), Some("a"));
        assert_eq!(parse("Shift+a").unwrap().text.as_deref(), Some("A"));
        assert!(parse("Hyper+A").is_err());
        assert!(parse("F13").is_err());
        assert!(parse("Bogus").is_err());
        assert_eq!(parse("Ctrl++").unwrap().code, KeyCode::Plus);
    }

    #[test]
    fn chord_names_round_trip() {
        for spec in ["Enter", "Ctrl+A", "Shift+Tab", "F5", "ArrowDown", "Ctrl+Shift+Z", "Escape", "Alt+F4"] {
            let c = parse(spec).unwrap();
            let name = chord_name(c.code, c.mods).unwrap();
            let back = parse(&name).unwrap();
            assert_eq!((back.code, back.mods), (c.code, c.mods), "{spec} -> {name}");
        }
        assert_eq!(chord_name(KeyCode::Unknown, ModifierKeys::default()), None);
    }
}
