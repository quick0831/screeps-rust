use std::collections::HashSet;

use js_sys::JsString;
use js_sys::Object;
use js_sys::Reflect;
use js_sys::global;
use log::error;
use log::info;
use screeps::game;
use wasm_bindgen::prelude::*;

// memory cleanup; memory gets created for all creeps upon spawning, and any time move_to
// is used; this should be removed if you're using RawMemory/serde for persistence
pub fn cleanup_memory() {
    info!("running memory cleanup");

    let alive_creeps: HashSet<String> = game::creeps().keys().collect();

    // grab `Memory` (if it exists)
    let Ok(memory) = Reflect::get(&global(), &JsString::from("Memory")) else {
        error!("Can't read global property: Memory");
        return;
    };

    // grab `Memory.creeps` (if it exists)
    let Ok(memory_creeps) = Reflect::get(&memory, &JsString::from("creeps")) else {
        error!("Can't read property of Memory: creeps");
        return;
    };

    // convert from JsValue to Object
    let memory_creeps: Object = memory_creeps.unchecked_into();

    // iterate memory creeps
    for creep_name_js in Object::keys(&memory_creeps).iter() {
        let creep_name = creep_name_js.as_string().unwrap();

        // check the HashSet for the creep name, deleting if not alive
        if !alive_creeps.contains(&creep_name) {
            info!("deleting memory for dead creep {}", creep_name);
            let _ = Reflect::delete_property(&memory_creeps, &creep_name_js);
        }
    }
}
