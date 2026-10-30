use js_sys::JsString;
use log::error;
use serde::{Serialize, de::DeserializeOwned};

pub struct MemoryRoot<T>(T);

impl<T: Default + DeserializeOwned> MemoryRoot<T> {
    pub fn get() -> Self {
        let raw = screeps::raw_memory::get();
        let raw_str = String::from(raw);

        let mem = serde_json::from_str(&raw_str);

        let mem = match mem {
            Ok(mem) => mem,
            Err(err) => {
                error!("Memory: {err}");
                Default::default()
            }
        };

        MemoryRoot(mem)
    }
}

impl<T: Serialize> MemoryRoot<T> {
    pub fn store(&mut self) {
        let mem = match serde_json::to_string(&self.0) {
            Ok(mem) => mem,
            Err(err) => {
                error!("Failed to serialize memory: {err}");
                return;
            }
        };
        let raw = JsString::from(mem);
        screeps::raw_memory::set(&raw);
    }
}

impl<T> AsMut<T> for MemoryRoot<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> AsRef<T> for MemoryRoot<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}
