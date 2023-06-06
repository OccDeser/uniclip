pub use device_query::Keycode;
use device_query::{DeviceQuery, DeviceState};

#[derive(Clone)]
pub struct HotKey {
    keys: Vec<Keycode>,
}

impl HotKey {
    pub fn new(keys: Vec<Keycode>) -> Self {
        let mut new_keys = Vec::new();
        // remove duplicates
        for key in keys {
            if !new_keys.contains(&key) {
                new_keys.push(key);
            }
        }

        // sort keys
        new_keys.sort_by(|a, b| {
            let a_number: u32 = *a as u32;
            let b_number: u32 = *b as u32;
            a_number.cmp(&b_number)
        });

        Self { keys: new_keys }
    }

    pub fn eq(&self, other: &Self) -> bool {
        if self.keys.len() != other.keys.len() {
            return false;
        }

        for i in 0..self.keys.len() {
            if self.keys[i] != other.keys[i] {
                return false;
            }
        }

        true
    }
}

pub struct ShortCut {
    hotkey: HotKey,
    callback: fn() -> (),
}

impl ShortCut {
    pub fn match_call(&self, hotkey: &HotKey) {
        if self.hotkey.eq(hotkey) {
            (self.callback)();
        }
    }
}

pub struct HotKeyManager {
    shortcuts: Vec<ShortCut>,
}

impl HotKeyManager {
    pub fn new() -> Self {
        Self {
            shortcuts: Vec::new(),
        }
    }

    pub fn register(&mut self, hotkey: HotKey, callback: fn() -> ()) {
        self.shortcuts.push(ShortCut { hotkey, callback });
    }

    pub fn listen(&self) {
        let mut last_key: HotKey = HotKey::new(Vec::new());
        let device_state: DeviceState = DeviceState::new();
        loop {
            // info!("listening hotkey...");
            std::thread::sleep(std::time::Duration::from_millis(100));
            let keys_down: Vec<Keycode> = device_state.get_keys();
            let now_key: HotKey = HotKey::new(keys_down.clone());
            // println!("keys: {:?}", now_key.keys);
            if !last_key.eq(&now_key) {
                last_key = now_key.clone();
                for shortcut in self.shortcuts.iter() {
                    shortcut.match_call(&now_key);
                }
            }
        }
    }
}
