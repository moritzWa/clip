//! Shortcut persistence and transactional native hotkey registration.
use global_hotkey::{
    GlobalHotKeyManager,
    hotkey::{Code, HotKey, Modifiers},
};
use std::{
    fs,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

pub const COMMAND: u8 = 1;
pub const SHIFT: u8 = 2;
pub const CONTROL: u8 = 4;
pub const OPTION: u8 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcut {
    pub key: u16,
    pub modifiers: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Config {
    pub history: Shortcut,
    pub images: Shortcut,
}

impl Shortcut {
    pub fn hotkey(self) -> Result<HotKey, String> {
        if self.modifiers & (COMMAND | CONTROL) == 0 || self.modifiers & !15 != 0 {
            return Err("Include Command or Control in the shortcut.".into());
        }
        let code = native_code(self.key)
            .ok_or("That key is not supported. Try a letter, number or function key.")?;
        let mut modifiers = Modifiers::empty();
        for (flag, value) in [
            (COMMAND, Modifiers::META),
            (SHIFT, Modifiers::SHIFT),
            (CONTROL, Modifiers::CONTROL),
            (OPTION, Modifiers::ALT),
        ] {
            if self.modifiers & flag != 0 {
                modifiers |= value;
            }
        }
        Ok(HotKey::new(Some(modifiers), code))
    }
}

impl Config {
    pub fn defaults(v_key: u16) -> Self {
        Self {
            history: Shortcut {
                key: v_key,
                modifiers: COMMAND | SHIFT,
            },
            images: Shortcut {
                key: v_key,
                modifiers: COMMAND | CONTROL,
            },
        }
    }
    pub fn legacy(period_key: u16) -> Self {
        Self {
            history: Shortcut {
                key: 13,
                modifiers: COMMAND | SHIFT,
            },
            images: Shortcut {
                key: period_key,
                modifiers: COMMAND,
            },
        }
    }
    fn hotkeys(self) -> Result<Vec<HotKey>, String> {
        let history = self.history.hotkey()?;
        let images = self.images.hotkey()?;
        if history.id() == images.id() {
            return Err("Choose different shortcuts for history and images.".into());
        }
        Ok(vec![history, images])
    }
    pub fn encode(self) -> String {
        format!(
            "version=1\nhistory_key={}\nhistory_modifiers={}\nimages_key={}\nimages_modifiers={}\n",
            self.history.key, self.history.modifiers, self.images.key, self.images.modifiers
        )
    }
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut values = std::collections::HashMap::new();
        for line in text.lines() {
            let (key, value) = line
                .split_once('=')
                .ok_or("Invalid shortcut settings file.")?;
            if values.insert(key, value).is_some() {
                return Err("Duplicate shortcut setting.".into());
            }
        }
        if values.len() != 5 || values.get("version") != Some(&"1") {
            return Err("Unsupported shortcut settings file.".into());
        }
        let number = |key| {
            values
                .get(key)
                .ok_or("Missing shortcut setting.")?
                .parse::<u16>()
                .map_err(|_| "Invalid shortcut key.".to_string())
        };
        let result = Self {
            history: Shortcut {
                key: number("history_key")?,
                modifiers: u8::try_from(number("history_modifiers")?)
                    .map_err(|_| "Invalid modifiers.")?,
            },
            images: Shortcut {
                key: number("images_key")?,
                modifiers: u8::try_from(number("images_modifiers")?)
                    .map_err(|_| "Invalid modifiers.")?,
            },
        };
        result.hotkeys()?;
        Ok(result)
    }
    fn save(self, path: &Path) -> Result<(), String> {
        let parent = path.parent().ok_or("Invalid settings path.")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
        let mut created = false;
        let result = (|| {
            let mut out = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)
                .map_err(|e| e.to_string())?;
            created = true;
            out.write_all(self.encode().as_bytes())
                .map_err(|e| e.to_string())?;
            out.sync_all().map_err(|e| e.to_string())?;
            fs::rename(&temporary, path).map_err(|e| e.to_string())
        })();
        // Only our own staged settings file is removed on a failed save.
        if result.is_err() && created {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

trait Registry {
    fn register(&self, key: HotKey) -> Result<(), String>;
    fn unregister(&self, key: HotKey) -> Result<(), String>;
}

fn load_config(path: &Path, initial: Config) -> (Config, Option<String>) {
    if path.exists() {
        match fs::read_to_string(path)
            .map_err(|e| e.to_string())
            .and_then(|s| Config::parse(&s))
        {
            Ok(config) => (config, None),
            Err(error) => (
                initial,
                Some(format!("Could not read saved shortcuts: {error}")),
            ),
        }
    } else {
        (initial, initial.save(path).err())
    }
}
impl Registry for GlobalHotKeyManager {
    fn register(&self, key: HotKey) -> Result<(), String> {
        self.register(key)
            .map_err(|e| format!("Shortcut could not be registered: {e}"))
    }
    fn unregister(&self, key: HotKey) -> Result<(), String> {
        self.unregister(key).map_err(|e| e.to_string())
    }
}

fn replace(registry: &impl Registry, old: &[HotKey], new: &[HotKey]) -> Result<(), String> {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    for &key in new
        .iter()
        .filter(|key| !old.iter().any(|old| old.id() == key.id()))
    {
        if let Err(error) = registry.register(key) {
            for key in added {
                let _ = registry.unregister(key);
            }
            return Err(error);
        }
        added.push(key);
    }
    for &key in old
        .iter()
        .filter(|key| !new.iter().any(|new| new.id() == key.id()))
    {
        if let Err(error) = registry.unregister(key) {
            for key in removed {
                let _ = registry.register(key);
            }
            for key in added {
                let _ = registry.unregister(key);
            }
            return Err(error);
        }
        removed.push(key);
    }
    Ok(())
}

pub struct Service {
    manager: GlobalHotKeyManager,
    pub config: Config,
    registered: Vec<HotKey>,
    path: PathBuf,
    pub issue: Option<String>,
}
impl Service {
    pub fn open(path: PathBuf, initial: Config) -> Result<Self, String> {
        let (config, mut issue) = load_config(&path, initial);
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        let keys = config.hotkeys()?;
        let registered = match replace(&manager, &[], &keys) {
            Ok(()) => keys,
            Err(error) => {
                issue = Some(error);
                Vec::new()
            }
        };
        Ok(Self {
            manager,
            config,
            registered,
            path,
            issue,
        })
    }
    pub fn history_id(&self) -> u32 {
        self.config.history.hotkey().unwrap().id()
    }
    pub fn images_id(&self) -> u32 {
        self.config.images.hotkey().unwrap().id()
    }
    pub fn accepts(&self, id: u32) -> bool {
        self.registered.iter().any(|key| key.id() == id)
    }
    pub fn suspend(&mut self) -> Result<(), String> {
        replace(&self.manager, &self.registered, &[])?;
        self.registered.clear();
        Ok(())
    }
    pub fn resume(&mut self) -> Result<(), String> {
        let next = self.config.hotkeys()?;
        replace(&self.manager, &self.registered, &next)?;
        self.registered = next;
        Ok(())
    }
    pub fn apply(&mut self, next: Config) -> Result<(), String> {
        let keys = next.hotkeys()?;
        replace(&self.manager, &self.registered, &keys)?;
        if let Err(error) = next.save(&self.path) {
            let _ = replace(&self.manager, &keys, &self.registered);
            return Err(format!("Could not save shortcuts: {error}"));
        }
        self.registered = keys;
        self.config = next;
        self.issue = None;
        Ok(())
    }
}

pub fn native_code(key: u16) -> Option<Code> {
    use Code::*;
    Some(match key {
        0 => KeyA,
        1 => KeyS,
        2 => KeyD,
        3 => KeyF,
        4 => KeyH,
        5 => KeyG,
        6 => KeyZ,
        7 => KeyX,
        8 => KeyC,
        9 => KeyV,
        10 => IntlBackslash,
        11 => KeyB,
        12 => KeyQ,
        13 => KeyW,
        14 => KeyE,
        15 => KeyR,
        16 => KeyY,
        17 => KeyT,
        18 => Digit1,
        19 => Digit2,
        20 => Digit3,
        21 => Digit4,
        22 => Digit6,
        23 => Digit5,
        24 => Equal,
        25 => Digit9,
        26 => Digit7,
        27 => Minus,
        28 => Digit8,
        29 => Digit0,
        30 => BracketRight,
        31 => KeyO,
        32 => KeyU,
        33 => BracketLeft,
        34 => KeyI,
        35 => KeyP,
        36 => Enter,
        37 => KeyL,
        38 => KeyJ,
        39 => Quote,
        40 => KeyK,
        41 => Semicolon,
        42 => Backslash,
        43 => Comma,
        44 => Slash,
        45 => KeyN,
        46 => KeyM,
        47 => Period,
        48 => Tab,
        49 => Space,
        50 => Backquote,
        51 => Backspace,
        64 => F17,
        65 => NumpadDecimal,
        67 => NumpadMultiply,
        69 => NumpadAdd,
        71 => NumLock,
        75 => NumpadDivide,
        76 => NumpadEnter,
        78 => NumpadSubtract,
        79 => F18,
        80 => F19,
        81 => NumpadEqual,
        82 => Numpad0,
        83 => Numpad1,
        84 => Numpad2,
        85 => Numpad3,
        86 => Numpad4,
        87 => Numpad5,
        88 => Numpad6,
        89 => Numpad7,
        90 => F20,
        91 => Numpad8,
        92 => Numpad9,
        93 => IntlYen,
        94 => IntlRo,
        95 => NumpadComma,
        96 => F5,
        97 => F6,
        98 => F7,
        99 => F3,
        100 => F8,
        101 => F9,
        103 => F11,
        105 => F13,
        106 => F16,
        107 => F14,
        109 => F10,
        111 => F12,
        113 => F15,
        114 => Insert,
        115 => Home,
        116 => PageUp,
        117 => Delete,
        118 => F4,
        119 => End,
        120 => F2,
        121 => PageDown,
        122 => F1,
        123 => ArrowLeft,
        124 => ArrowRight,
        125 => ArrowDown,
        126 => ArrowUp,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_legacy_keep_distinct_shortcuts() {
        let defaults = Config::defaults(9);
        assert_eq!(defaults.history.key, 9);
        assert_eq!(defaults.images.modifiers, COMMAND | CONTROL);
        let legacy = Config::legacy(15);
        assert_eq!(legacy.history.key, 13);
        assert_eq!(legacy.images.key, 15);
        assert_eq!(Config::parse(&legacy.encode()).unwrap(), legacy);
    }
    #[test]
    fn invalid_and_duplicate_shortcuts_are_rejected() {
        let mut config = Config::defaults(9);
        config.images = config.history;
        assert!(config.hotkeys().is_err());
        assert!(
            Shortcut {
                key: 9,
                modifiers: SHIFT
            }
            .hotkey()
            .is_err()
        );
        assert!(
            Shortcut {
                key: 200,
                modifiers: COMMAND
            }
            .hotkey()
            .is_err()
        );
        assert!(Config::parse("version=9").is_err());
    }
    #[test]
    fn failed_registration_rolls_back_without_losing_old_keys() {
        use std::cell::RefCell;
        struct Fake {
            keys: RefCell<Vec<u32>>,
            fail: u32,
        }
        impl Registry for Fake {
            fn register(&self, key: HotKey) -> Result<(), String> {
                if key.id() == self.fail {
                    return Err("in use".into());
                }
                self.keys.borrow_mut().push(key.id());
                Ok(())
            }
            fn unregister(&self, key: HotKey) -> Result<(), String> {
                self.keys.borrow_mut().retain(|id| *id != key.id());
                Ok(())
            }
        }
        let old = Config::legacy(47).hotkeys().unwrap();
        let next = Config::defaults(9).hotkeys().unwrap();
        let initial = old.iter().map(|k| k.id()).collect::<Vec<_>>();
        let registry = Fake {
            keys: RefCell::new(initial.clone()),
            fail: next[1].id(),
        };
        assert!(replace(&registry, &old, &next).is_err());
        assert_eq!(*registry.keys.borrow(), initial);
    }
    #[test]
    fn config_file_roundtrips_and_corruption_is_not_overwritten() {
        let dir =
            std::env::temp_dir().join(format!("clipglass-shortcut-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("shortcuts.conf");
        let config = Config::defaults(9);
        config.save(&path).unwrap();
        assert_eq!(
            Config::parse(&fs::read_to_string(&path).unwrap()).unwrap(),
            config
        );
        fs::write(&path, "invalid settings").unwrap();
        assert!(Config::parse(&fs::read_to_string(&path).unwrap()).is_err());
        let (loaded, issue) = load_config(&path, config);
        assert_eq!(loaded, config);
        assert!(issue.is_some());
        assert_eq!(fs::read_to_string(&path).unwrap(), "invalid settings");
        fs::remove_file(path).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
