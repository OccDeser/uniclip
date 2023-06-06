static ALERT_LEVEL: u8 = 2;

pub fn alert(prompt: &str, message: String, level: u8) {
    if level <= ALERT_LEVEL {
        eprintln!("[{}]: {}", prompt, message);
    }
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        use crate::alert::alert;
        alert("DEBUG", format!($($arg)*), 3);
    })
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ({
        use crate::alert::alert;
        alert("INFO", format!($($arg)*), 2);
    })
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ({
        use crate::alert::alert;
        alert("WARN", format!($($arg)*), 1);
    })
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ({
        use crate::alert::alert;
        alert("ERROR", format!($($arg)*), 0);
    })
}
