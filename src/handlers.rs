use crate::app::App;
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_seat, delegate_shm,
    output::OutputHandler,
    registry::ProvidesRegistryState,
    seat::{Capability, SeatHandler},
    shell::{wlr_layer::{LayerShellHandler, LayerSurface, LayerSurfaceConfigure}, WaylandSurface},
    shm::{Shm, ShmHandler},
    output::OutputState,
    registry::RegistryState,
    seat::SeatState,
    registry_handlers,
};
use wayland_client::{
    protocol::{wl_output, wl_seat, wl_surface},
    Connection, QueueHandle, Proxy,
};

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) { self.draw(); }
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        let output_name = self.output_state.info(&output).and_then(|info| info.name.clone());
        let matches_target = output_name.as_ref().map_or(false, |name| {
            self.target_display_name == "default" || name == &self.target_display_name
        });

        if matches_target {
            let different_output = self.bound_output.as_ref().map_or(true, |b| b.id() != output.id());
            if self.layer.is_none() || different_output {
                self.bound_output = Some(output);
                self.needs_recreation = true;
            }
        }
    }

    fn update_output(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if let Some(bound) = &self.bound_output {
            if bound.id() == output.id() {
                if self.width > 0 && self.height > 0 && self.layer.is_none() {
                    self.recreate_layer_surface(qh, Some(output));
                } else if self.width == 0 || self.height == 0 {
                    if let Some(layer) = &self.layer { layer.commit(); }
                }
            }
        }
    }

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if let Some(bound) = &self.bound_output {
            if bound.id() == output.id() {
                self.bound_output = None;
                self.needs_recreation = true;
            }
        }
    }
}

impl LayerShellHandler for App {
    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface, configure: LayerSurfaceConfigure, _: u32) {
        let (w, h) = configure.new_size;
        let (new_width, new_height) = (w as i32, h as i32);

        let was_zero = self.width == 0 || self.height == 0;
        self.width = new_width;
        self.height = new_height;

        if new_width == 0 || new_height == 0 { return; }

        if was_zero {
            layer.set_anchor(smithay_client_toolkit::shell::wlr_layer::Anchor::TOP
                | smithay_client_toolkit::shell::wlr_layer::Anchor::BOTTOM
                | smithay_client_toolkit::shell::wlr_layer::Anchor::LEFT
                | smithay_client_toolkit::shell::wlr_layer::Anchor::RIGHT);
            layer.set_margin(-1, -1, -1, -1);
            layer.set_exclusive_zone(-1);
            layer.commit();
        }

        self.draw();
    }

    fn closed(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, _: &LayerSurface) {
        let target_output = self.output_state.outputs().find(|output| {
            self.output_state.info(output)
                .and_then(|info| info.name.clone())
                .map_or(false, |name| self.target_display_name == "default" || name == self.target_display_name)
        });
        self.recreate_layer_surface(qh, target_output);
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
    fn new_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, _: Capability) {}
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, _: Capability) {}
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ShmHandler for App { fn shm_state(&mut self) -> &mut Shm { &mut self.shm } }

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }

    // The macro generates runtime_add_global / runtime_remove_global automatically
    registry_handlers![OutputState, SeatState];
}

delegate_compositor!(App);
delegate_output!(App);
delegate_seat!(App);
delegate_shm!(App);
delegate_layer!(App);
delegate_registry!(App);
