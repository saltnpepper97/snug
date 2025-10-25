use crate::args::MergedConfig;
use crate::colour::parse_colour;
use crate::drawing::draw_snug;
use smithay_client_toolkit::{
    compositor::{CompositorState, Region},
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    shell::{wlr_layer::{Anchor, KeyboardInteractivity, Layer, LayerShell, LayerSurface}, WaylandSurface},
    shm::{slot::SlotPool, Shm},
};
use wayland_client::{protocol::wl_output, QueueHandle};

pub struct App {
    pub registry_state: RegistryState,
    pub output_state: OutputState,
    pub seat_state: SeatState,
    pub compositor_state: CompositorState,
    pub layer_shell: LayerShell,
    pub shm: Shm,
    pub pool: Option<SlotPool>,
    pub layer: Option<LayerSurface>,
    pub width: i32,
    pub height: i32,
    pub config: MergedConfig,
    
    // Track which output we're bound to
    pub bound_output: Option<wl_output::WlOutput>,
    pub target_display_name: String,
    pub needs_recreation: bool,
}

impl App {
    pub fn draw(&mut self) {
        let Some(pool) = self.pool.as_mut() else { 
            eprintln!("[{}] draw() called but pool is None", self.target_display_name);
            return;
        };
        let Some(layer) = &self.layer else {
            eprintln!("[{}] draw() called but layer is None", self.target_display_name); 
            return;
        };
        if self.width == 0 || self.height == 0 {
            eprintln!("[{}] draw() called but dimensions are zero: {}x{}", 
                     self.target_display_name, self.width, self.height);
            return;
        }
        
        eprintln!("[{}] Drawing with dimensions {}x{}", self.target_display_name, self.width, self.height);
        
        let stride = self.width * 4;
        let (buffer, canvas) = match pool.create_buffer(
            self.width, 
            self.height, 
            stride, 
            wayland_client::protocol::wl_shm::Format::Argb8888
        ) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[{}] Failed to create buffer: {:?}", self.target_display_name, e);
                return;
            }
        };
        
        let (r, g, b, a) = parse_colour(&self.config.color, self.config.opacity);
        draw_snug(canvas, self.width, self.height, r, g, b, a, &self.config);
        
        let surface = layer.wl_surface();
        
        // Set input region to only the border areas
        let region = match Region::new(&self.compositor_state) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[{}] Failed to create region: {:?}", self.target_display_name, e);
                return;
            }
        };
        
        let left = self.config.left;
        let right = self.config.right;
        let top = self.config.top;
        let bottom = self.config.bottom;
        
        // Top border
        region.add(0, 0, self.width, top);
        // Bottom border
        region.add(0, self.height - bottom, self.width, bottom);
        // Left border (excluding corners already covered)
        region.add(0, top, left, self.height - top - bottom);
        // Right border (excluding corners already covered)
        region.add(self.width - right, top, right, self.height - top - bottom);
        
        surface.set_input_region(Some(region.wl_region()));
        
        surface.attach(Some(buffer.wl_buffer()), 0, 0);
        surface.damage_buffer(0, 0, self.width, self.height);
        surface.commit();
        
        eprintln!("[{}] Draw complete - buffer attached and committed", self.target_display_name);
    }
    
    pub fn recreate_layer_surface(&mut self, qh: &QueueHandle<Self>, output: Option<wl_output::WlOutput>) {
        eprintln!("[{}] Recreating layer surface...", self.target_display_name);
        
        // Destroy old layer surface if it exists
        if let Some(old_layer) = self.layer.take() {
            eprintln!("[{}] Dropping old layer surface", self.target_display_name);
            drop(old_layer);
        }
        
        // CRITICAL: Recreate the buffer pool too!
        // The old pool might be tied to the old surface or invalid after DPMS
        eprintln!("[{}] Creating new buffer pool", self.target_display_name);
        match SlotPool::new(256 * 256 * 4, &self.shm) {
            Ok(new_pool) => {
                self.pool = Some(new_pool);
                eprintln!("[{}] New buffer pool created successfully", self.target_display_name);
            },
            Err(e) => {
                eprintln!("[{}] Failed to create new buffer pool: {:?}", self.target_display_name, e);
                return;
            }
        }
        
        // Create new surface
        let surface = self.compositor_state.create_surface(qh);
        eprintln!("[{}] New wl_surface created", self.target_display_name);
        
        // Create new layer surface bound to the output
        let layer = self.layer_shell.create_layer_surface(
            qh,
            surface,
            Layer::Top,
            Some("snug-overlay"),
            output.as_ref(),
        );
        eprintln!("[{}] New layer surface created", self.target_display_name);
        
        // Configure the layer surface with explicit settings
        layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
        layer.set_margin(-1, -1, -1, -1);
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.commit();
        eprintln!("[{}] Layer surface configured and committed", self.target_display_name);
        
        // Store the new layer and output reference
        self.layer = Some(layer);
        self.bound_output = output;
        self.needs_recreation = false;
        
        // Reset dimensions - will be set by configure event
        self.width = 0;
        self.height = 0;
        
        eprintln!("[{}] Recreation complete, waiting for configure event", self.target_display_name);
    }
}
