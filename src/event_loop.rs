use crate::app::App;
use crate::args::Args;
use crate::config::{load_config_silent, load_config_or_default};
use crate::wayland;
use crate::process::release_lock;
use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{wlr_layer::LayerShell, WaylandSurface},
    shm::Shm,
};
use wayland_client::{globals::registry_queue_init, Proxy};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use notify::{Watcher, RecursiveMode, Event};

/// Main event loop for a child process
pub fn run_event_loop(
    args: Args,
    running: Arc<AtomicBool>,
) -> Result<(), Box<dyn std::error::Error>> {
    let target_display = args.display.as_ref().unwrap(); 
    
    // Load config using the path from args if provided, otherwise use default search
    let snug_config = if let Some(path) = &args.config {
        load_config_silent(path)?
    } else {
        load_config_or_default()
    };
    
    // Get config for this display and merge with CLI args    
    let display_config = snug_config.get_display_config(target_display);

    let merged_config = args.merge_with_config(&display_config);
    
    // Create Wayland connection with retry logic
    let conn = wayland::create_wayland_connection(target_display)?;
    
    let (globals, mut event_queue) = registry_queue_init(&conn)?;
    let qh = event_queue.handle();
    
    // Create a temporary App to query outputs
    let mut temp_app = App {
        registry_state: RegistryState::new(&globals),
        output_state: OutputState::new(&globals, &qh),
        seat_state: SeatState::new(&globals, &qh),
        compositor_state: CompositorState::bind(&globals, &qh)?,
        layer_shell: LayerShell::bind(&globals, &qh)?,
        shm: Shm::bind(&globals, &qh)?,
        pool: None,
        layer: None,
        width: 0,
        height: 0,
        config: merged_config.clone(),
        bound_output: None,
        target_display_name: target_display.clone(),
        needs_recreation: false,
    };
    
    // Dispatch events to populate output_state
    event_queue.roundtrip(&mut temp_app)?;
    
    // Find the matching output by name
    let target_output = wayland::find_target_output(&mut temp_app, target_display);
    
    if target_output.is_none() {
        eprintln!("Warning: Could not find output '{}', exiting", target_display);
        release_lock(target_display);
        return Ok(());
    }
    
    // Set up the layer surface
    let (pool, layer) = wayland::setup_layer_surface(&mut temp_app, target_output.clone(), &qh)?;
    
    // Now create the real App with the layer surface
    let mut app = App {
        registry_state: temp_app.registry_state,
        output_state: temp_app.output_state,
        seat_state: temp_app.seat_state,
        compositor_state: temp_app.compositor_state,
        layer_shell: temp_app.layer_shell,
        shm: temp_app.shm,
        pool: Some(pool),
        layer: Some(layer),
        width: 0,
        height: 0,
        config: merged_config,
        bound_output: target_output.clone(),
        target_display_name: target_display.clone(),
        needs_recreation: false,
    };
    
    conn.flush()?;
    
    // Wait for configure event
    let mut configured = false;
    while !configured {
        event_queue.blocking_dispatch(&mut app)?;
        configured = app.width > 0 && app.height > 0;
    }
    
    conn.flush()?;
    if let Some(layer) = &app.layer {
        layer.commit();
    }
    
    // Set up config hot reload
    let config_needs_reload = Arc::new(Mutex::new(false));
    setup_config_watcher(
        config_needs_reload.clone(),
        running.clone(),
        args.config.clone(), // FIXED: Pass the custom config path
    );
    
    // Run the main loop
    main_loop(
        app,
        event_queue,
        conn,
        args,
        running,
        config_needs_reload,
    )
}

/// Set up file watcher for config hot reload
fn setup_config_watcher(
    config_needs_reload: Arc<Mutex<bool>>,
    running: Arc<AtomicBool>,
    custom_config_path: Option<String>, // FIXED: Accept custom config path
) {
    thread::spawn(move || {
        let config_path = custom_config_path
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| crate::config::get_config_path());
        
        eprintln!("Watching config file: {}", config_path.display());
        
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                if event.kind.is_modify() {
                    let _ = tx.send(());
                }
            }
        }).expect("Failed to create file watcher");
        
        if watcher.watch(&config_path, RecursiveMode::NonRecursive).is_err() {
            // Config file might not exist yet, silently continue
            eprintln!("Config file does not exist yet: {}", config_path.display());
            return;
        }
        
        while running.load(Ordering::SeqCst) {
            if rx.recv().is_ok() {
                // Debounce multiple events
                thread::sleep(Duration::from_millis(100));
                while rx.try_recv().is_ok() {}
                
                eprintln!("Config file changed, reloading...");
                *config_needs_reload.lock().unwrap() = true;
            }
        }
    });
}

/// Main event loop with config reload and surface lifecycle management
fn main_loop(
    mut app: App,
    mut event_queue: wayland_client::EventQueue<App>,
    conn: wayland_client::Connection,
    cli_args: Args,
    running: Arc<AtomicBool>,
    config_needs_reload: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let display_name = app.target_display_name.clone();
    let qh = event_queue.handle();

    let mut last_dimensions = (app.width, app.height);
    let mut was_suspended = false;
    let mut resume_time: Option<Instant> = None;
    let mut last_draw_time = Instant::now();

    loop {
        if !running.load(Ordering::SeqCst) {
            eprintln!("Compositor connection lost, exiting...");
            break Ok(());
        }

        // Config hot reload
        if *config_needs_reload.lock().unwrap() {
            // FIXED: Load from custom config path if provided
            let new_config = if let Some(path) = &cli_args.config {
                match load_config_silent(path) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("Failed to reload custom config from {}: {}", path, e);
                        *config_needs_reload.lock().unwrap() = false;
                        continue;
                    }
                }
            } else {
                load_config_or_default()
            };
            
            app.config = cli_args.merge_with_config(&new_config.get_display_config(&display_name));
            if app.width > 0 && app.height > 0 {
                app.draw();
                conn.flush()?;
                last_draw_time = Instant::now();
                eprintln!("Config reloaded and redrawn");
            }
            *config_needs_reload.lock().unwrap() = false;
        }

        // Check if bound output disappeared (zombie layer)
        if let Some(bound) = &app.bound_output {
            let output_exists = app.output_state.outputs().any(|o| o.id() == bound.id());
            if !output_exists {
                eprintln!("[{}] Zombie surface detected, clearing and searching by name...", display_name);
                app.layer = None;
                app.bound_output = None;
                app.width = 0;
                app.height = 0;

                for output in app.output_state.outputs() {
                    if let Some(info) = app.output_state.info(&output) {
                        if info.name.as_deref() == Some(&display_name) {
                            app.recreate_layer_surface(&qh, Some(output.clone()));
                            wait_for_configure(&mut event_queue, &mut app, 30)?;
                            app.draw();
                            conn.flush()?;
                            resume_time = Some(Instant::now());
                            last_draw_time = Instant::now();
                            break;
                        }
                    }
                }
            }
        }

        // Needs recreation triggered elsewhere
        if app.needs_recreation {
            if let Some(output) = app.bound_output.clone() {
                app.recreate_layer_surface(&qh, Some(output));
                wait_for_configure(&mut event_queue, &mut app, 30)?;
                app.draw();
                conn.flush()?;
                resume_time = Some(Instant::now());
                last_draw_time = Instant::now();
                app.needs_recreation = false;
            }
        }

        // Exit if no outputs exist
        if app.bound_output.is_none() && app.output_state.outputs().next().is_none() {
            release_lock(&display_name);
            std::process::exit(0);
        }

        // Detect dimension changes (suspend/resume)
        let current_dimensions = (app.width, app.height);
        if current_dimensions != last_dimensions {
            if current_dimensions.0 == 0 || current_dimensions.1 == 0 {
                was_suspended = true;
                resume_time = None;
            } else if was_suspended {
                force_layer_recommit(&app);
                for _ in 0..3 {
                    thread::sleep(Duration::from_millis(100));
                    app.draw();
                    conn.flush()?;
                }
                was_suspended = false;
                resume_time = Some(Instant::now());
                last_draw_time = Instant::now();
            } else {
                app.draw();
                conn.flush()?;
                last_draw_time = Instant::now();
            }
            last_dimensions = current_dimensions;
        }

        // Recreate layer if lost during DPMS
        if app.layer.is_none() && app.width > 0 && app.height > 0 && app.bound_output.is_some() {
            eprintln!("[{}] Layer surface lost, recreating...", display_name);
            app.recreate_layer_surface(&qh, app.bound_output.clone());
            wait_for_configure(&mut event_queue, &mut app, 20)?;
            app.draw();
            conn.flush()?;
            last_draw_time = Instant::now();
        }

        // High-refresh post-resume redraws
        if let Some(resume) = resume_time {
            if resume.elapsed() < Duration::from_secs(10) && last_draw_time.elapsed() > Duration::from_secs(2) {
                if app.width > 0 && app.height > 0 {
                    app.draw();
                    conn.flush()?;
                    last_draw_time = Instant::now();
                }
            } else {
                resume_time = None;
            }
        }

        let _ = event_queue.dispatch_pending(&mut app);
        thread::sleep(Duration::from_millis(50));
    }
}

/// Helper to wait for configure events
fn wait_for_configure(
    event_queue: &mut wayland_client::EventQueue<App>,
    app: &mut App,
    retries: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    for _ in 0..retries {
        event_queue.blocking_dispatch(app)?;
        if app.width > 0 && app.height > 0 { break; }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

/// Helper to recommit layer after resume
fn force_layer_recommit(app: &App) {
    if let Some(layer) = &app.layer {
        use smithay_client_toolkit::shell::wlr_layer::Anchor;
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_margin(-1, -1, -1, -1);
        layer.set_exclusive_zone(-1);
        layer.commit();
    }
}
