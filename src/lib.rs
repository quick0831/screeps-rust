use screeps::game;
use wasm_bindgen::prelude::*;

#[cfg(feature = "mmo")]
use log::info;
use log::warn;

use crate::memory::MemoryRoot;
use crate::shard::ShardMemory;

mod construction_alloc;
mod container;
mod logging;
mod memory;
mod metric;
mod path_finder;
mod roles;
mod room;
mod shard;
mod source;
mod source_alloc;
mod spawn;
mod tower;
mod transport_alloc;
mod utils;

static INIT_LOGGING: std::sync::Once = std::sync::Once::new();

#[wasm_bindgen(js_name = loop)]
pub fn game_loop() {
    INIT_LOGGING.call_once(|| {
        // show all output of Info level, adjust as needed
        logging::setup_logging(logging::Info);
    });

    let time = game::time();

    let mut memory: MemoryRoot<ShardMemory> = MemoryRoot::get();
    let shard = memory.as_mut();
    shard.process(time);
    memory.store();

    if time.is_multiple_of(100) {
        memory::cleanup_memory();
    }

    let cpu_limit = game::cpu::limit();
    let cpu_usage = game::cpu::get_used();

    if cpu_usage.floor() as u32 > cpu_limit {
        warn!("Detect CPU spike: {cpu_usage:.2}/{cpu_limit}");
    }

    #[cfg(feature = "mmo")]
    if game::cpu::bucket() >= screeps::PIXEL_CPU_COST as i32 {
        match game::cpu::generate_pixel() {
            Ok(()) => info!("Generated pixel!"),
            Err(err) => warn!("Generate pixel failed: {err}"),
        }
    }
}
