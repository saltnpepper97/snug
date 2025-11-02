use crate::args::Args;
use crate::config::{load_config, load_config_or_default};
use crate::wayland;
use crate::event_loop;
use std::env;
use std::fs;
use std::io::Write;
use std::process::Command;
use std::os::unix::process::CommandExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use nix::sys::prctl::set_pdeathsig;
use nix::sys::signal::Signal;

/// Automatically releases the display lock on drop
pub struct LockGuard {
    display: String,
}

impl LockGuard {
    pub fn new(display: &str) -> Option<Self> {
        if try_acquire_lock(display).is_ok() {
            Some(Self { display: display.to_string() })
        } else {
            None
        }
    }
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        release_lock(&self.display);
    }
}

// Helper to expand ~ in paths
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Parent process: spawn a child for each configured display
pub fn spawn_child_processes(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    // Expand tilde in config path ONCE in parent
    let expanded_config_path = args.config.as_ref().map(|p| expand_tilde(p));
    
    // Load config using the expanded path
    let snug_config = if let Some(path) = &expanded_config_path {
        eprintln!("Loading config from: {}", path);
        load_config(path)?
    } else {
        load_config_or_default()
    };
    
    let exe_path = env::current_exe()?;
    
    let mut spawned = 0;
    
    // Spawn a child process for each configured display
    for display_name in snug_config.displays.keys() {
        if display_name == "default" {
            continue;
        }
        
        // Check if instance already running for this display
        if try_acquire_lock(display_name).is_err() {
            eprintln!("Instance already running for display '{}', skipping", display_name);
            continue;
        }
        // Release the parent's lock immediately - child will acquire its own
        release_lock(display_name);
       
        unsafe {
            let mut cmd = Command::new(&exe_path);
            cmd.arg("--display").arg(display_name);
            
            // Pass EXPANDED config path to child
            if let Some(config_path) = &expanded_config_path {
                cmd.arg("-c").arg(config_path);
            }
            
            cmd.pre_exec(|| {
                // Kill child if parent dies
                set_pdeathsig(Some(Signal::SIGTERM))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(())
            })
            .spawn()?;
        }

        spawned += 1;
    }
    
    if spawned == 0 {
        eprintln!("No displays configured or all instances already running");
        return Ok(());
    }
    
    // Monitor Wayland compositor instead of sleeping forever
    wayland::monitor_wayland_compositor();
    
    Ok(())
}

/// Child process: run for a specific display
pub fn run_child_process(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let target_display = args.display.as_ref().unwrap();
    
    // Try to acquire lock for this display, automatically release on drop
    let _lock_guard = match LockGuard::new(target_display) {
        Some(g) => g,
        None => {
            eprintln!("Another instance is already running for display '{}'", target_display);
            return Ok(());
        }
    };

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Install Ctrl-C / termination handler
    ctrlc::set_handler(move || {
        eprintln!("Received termination signal, shutting down...");
        r.store(false, Ordering::SeqCst);
    })?;

    // Spawn Wayland compositor monitor for child process
    let r2 = running.clone();
    std::thread::spawn(move || {
        wayland::monitor_wayland_compositor_with_flag(r2);
    });

    // Run the main event loop
    event_loop::run_event_loop(args, running)?;
    
    Ok(())
}

fn get_lock_file_path(display_name: &str) -> std::path::PathBuf {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    std::path::PathBuf::from(runtime_dir).join(format!("snug-{}.lock", display_name))
}

fn try_acquire_lock(display_name: &str) -> Result<fs::File, std::io::Error> {
    let lock_path = get_lock_file_path(display_name);
    
    // Check if file exists
    if let Ok(contents) = fs::read_to_string(&lock_path) {
        if let Ok(pid) = contents.trim().parse::<u32>() {
            // check if PID is alive
            if nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok() {
                // process still exists
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!("Process {} already holds the lock", pid)
                ));
            } else {
                // stale lock: remove it
                let _ = fs::remove_file(&lock_path);
            }
        } else {
            // corrupted lock: remove it
            let _ = fs::remove_file(&lock_path);
        }
    }

    // Try to create the lock file
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
        .and_then(|mut file| {
            writeln!(file, "{}", std::process::id())?;
            Ok(file)
        })
}

pub fn release_lock(display_name: &str) {
    let lock_path = get_lock_file_path(display_name);
    let _ = fs::remove_file(lock_path);
}
