#![allow(dead_code)]

use notify_rust::{Notification, Timeout};

pub fn notify(summary: &str, body: &str) {
    if let Err(e) = Notification::new()
        .appname("Tomato")
        .summary(summary)
        .body(body)
        .timeout(Timeout::Milliseconds(6000))
        .show()
    {
        eprintln!("tomato: failed to send notification: {e}");
    }
}
