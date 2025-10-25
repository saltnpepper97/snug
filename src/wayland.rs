use crate::app::App;
use crate::process::release_lock;
use smithay_client_toolkit::{
    shell::{wlr_layer::{Anchor, KeyboardInteractivity, Layer}, WaylandSurface},
    shm::{slot::SlotPool},
};
use wayland_client::{Connection, protocol::wl_output};
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Monitor Wayland compositor socket - exit when it disappears (for parent process)
pub fn monitor_wayland_compositor() {
    let socket_path = get_wayland_socket_path();
    
    loop {
        thread::sleep(Duration::from_secs(2));
        
        if !std::path::Path::new(&socket_path).exists() {
            eprintln!("Wayland compositor socket disappeared, parent exiting...");
            std::process::exit(0);
        }
    }
}

/// Monitor Wayland compositor socket with flag (for child processes)
pub fn monitor_wayland_compositor_with_flag(running: Arc<AtomicBool>) {
    let socket_path = get_wayland_socket_path();
    
    loop {
        thread::sleep(Duration::from_secs(2));
        
        if !std::path::Path::new(&socket_path).exists() {
            eprintln!("Wayland compositor socket disappeared, shutting down...");
            running.store(false, Ordering::SeqCst);
            break;
        }
    }
}

/// Get the Wayland socket path
pub fn get_wayland_socket_path() -> String {
    let wayland_display = env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string());
    let xdg_runtime = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/run/user/1000".to_string());
    format!("{}/{}", xdg_runtime, wayland_display)
}

/// Create a Wayland connection with retry logic
pub fn create_wayland_connection(target_display: &str) -> Result<Connection, wayland_client::ConnectError> {
    let mut retries = 0;
    loop {
        match Connection::connect_to_env() {
            Ok(c) => return Ok(c),
            Err(_e) if retries < 10 => {
                retries += 1;
                thread::sleep(Duration::from_millis(500));
                continue;
            }
            Err(e) => {
                release_lock(target_display);
                return Err(e);
            }
        }
    }
}

/// Find the target output by name
pub fn find_target_output(
    temp_app: &mut App,
    target_display: &str,
) -> Option<wl_output::WlOutput> {
    for output in temp_app.output_state.outputs() {
        if let Some(info) = temp_app.output_state.info(&output) {
            if let Some(name) = info.name.as_ref() {
                if name == target_display || (target_display == "default" && temp_app.bound_output.is_none()) {
                    eprintln!("Found target output: {}", name);
                    if target_display != "default" {
                        return Some(output.clone()); // Exact match found
                    }
                    // For "default", keep looking for first available
                    return Some(output.clone());
                }
            }
        }
    }
    None
}

/// Create the initial layer surface and app setup
pub fn setup_layer_surface(
    temp_app: &mut App,
    target_output: Option<wl_output::WlOutput>,
    qh: &wayland_client::QueueHandle<App>,
) -> Result<(SlotPool, smithay_client_toolkit::shell::wlr_layer::LayerSurface), Box<dyn std::error::Error>> {
    let pool = SlotPool::new(2 * 1024 * 1024, &temp_app.shm)?;
    let surface = temp_app.compositor_state.create_surface(qh);
    
    // Bind to specific output
    let layer = temp_app.layer_shell.create_layer_surface(
        qh, 
        surface, 
        Layer::Top, 
        Some("snug-overlay"), 
        target_output.as_ref()
    );
    
    layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
    layer.set_margin(-1, -1, -1, -1);
    layer.set_exclusive_zone(-1);
    layer.set_keyboard_interactivity(KeyboardInteractivity::None);
    layer.commit();
    
    Ok((pool, layer))
}
