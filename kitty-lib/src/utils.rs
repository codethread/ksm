use log::{debug, warn};
use std::env;
use std::process::Command;

pub fn get_kitty_socket() -> String {
    if let Ok(socket) = env::var("KITTY_LISTEN_ON") {
        debug!("Using KITTY_LISTEN_ON environment variable: {socket}");
        return socket;
    }

    debug!("KITTY_LISTEN_ON not set, searching for socket files");

    // Find socket file
    if let Ok(output) = Command::new("sh")
        .arg("-c")
        .arg("ls /tmp/mykitty* 2>/dev/null | head -1")
        .output()
    {
        if let Ok(socket_file) = String::from_utf8(output.stdout) {
            let socket_file = socket_file.trim();
            if !socket_file.is_empty() {
                let socket_path = format!("unix:{}", socket_file);
                debug!("Found socket file: {}", socket_path);
                return socket_path;
            }
        }
    }

    let default_socket = "unix:/tmp/mykitty".to_string();
    warn!("No socket file found, using default: {}", default_socket);
    default_socket
}
