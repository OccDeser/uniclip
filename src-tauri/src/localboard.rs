use arboard::Clipboard;

pub fn get_localboard() -> String {
    let mut clipboard: Clipboard = Clipboard::new().unwrap();
    clipboard.get_text().unwrap()
}

pub fn set_localboard(text: String) {
    let mut clipboard: Clipboard = Clipboard::new().unwrap();
    clipboard.set_text(text).unwrap();
}
