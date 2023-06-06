#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod alert;
mod clipboard;
mod hotkey;
mod liaison;
mod localboard;

use std::sync::Arc;
use std::thread;

use lazy_static::lazy_static;

use tokio::runtime::Runtime;
use tokio::sync::{Mutex, MutexGuard};

use clipboard::Clipboard;
use hotkey::{HotKey, HotKeyManager, Keycode};
use liaison::Liaison;
use localboard::{get_localboard, set_localboard};

lazy_static! {
    static ref PORT: u16 = 1699;
    static ref IP: String = "192.168.1.164".to_string();
    static ref CLIPBOARD: Arc<Mutex<Clipboard>> = Arc::new(Mutex::new(Clipboard::new(16)));
    static ref LIAISON: Arc<Mutex<Liaison>> =
        Liaison::new(IP.as_str(), PORT.clone(), CLIPBOARD.clone()).unwrap();
    static ref STAERTED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

#[tauri::command]
async fn start_liaison() {
    let started = STAERTED.lock().await;
    if *started {
        return;
    }

    info!("Starting liaison");
    Liaison::start(LIAISON.clone()).await;

    let mut started: MutexGuard<bool> = STAERTED.lock().await;
    *started = true;
    drop(started);
}

#[tauri::command]
fn clipboard_get() -> Vec<String> {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let clipboard: MutexGuard<Clipboard> = CLIPBOARD.lock().await;
        let content = clipboard.content.clone();
        drop(clipboard);
        let mut cliplist: Vec<String> = Vec::new();
        for c in content.iter() {
            let clip: &str = std::str::from_utf8(c).unwrap();
            cliplist.push(clip.to_string());
        }
        cliplist
    })
}

#[tauri::command]
async fn clipboard_broadcast(message: String) {
    info!("Broadcasting message: {}", message);
    Liaison::broadcast(LIAISON.clone(), message.as_bytes()).await;
    let mut clipboard: MutexGuard<Clipboard> = CLIPBOARD.lock().await;
    clipboard.append(message.as_bytes().to_vec());
    drop(clipboard);
}

fn press_alt_c() {
    info!("Pressing Alt_C");

    let data: String = get_localboard();
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        clipboard_broadcast(data).await;
    })
}

fn press_alt_v() {
    info!("Pressing Alt_V");

    let cliplist: Vec<String> = clipboard_get();
    set_localboard(cliplist[0].clone());
}

fn start_hotkey() {
    thread::spawn(|| {
        let mut hotkey_manager: HotKeyManager = HotKeyManager::new();
        let hotkey: HotKey = HotKey::new(vec![Keycode::LAlt, Keycode::C]);
        hotkey_manager.register(hotkey, press_alt_c);
        let hotkey: HotKey = HotKey::new(vec![Keycode::LAlt, Keycode::V]);
        hotkey_manager.register(hotkey, press_alt_v);
        hotkey_manager.listen();
    });
}

fn main() {
    start_hotkey();

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            start_liaison,
            clipboard_get,
            clipboard_broadcast
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
