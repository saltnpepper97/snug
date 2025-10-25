mod app;
mod args;
mod colour;
mod config;
mod drawing;
mod handlers;
mod process;
mod wayland;
mod event_loop;

use args::Args;
use clap::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    if args.display.is_none() {
        process::spawn_child_processes(args)
    } else {
        process::run_child_process(args)
    }
}
