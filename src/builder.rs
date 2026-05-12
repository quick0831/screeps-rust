use screeps::Creep;
use serde::{Deserialize, Serialize};

use crate::SharedData;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuilderMemory {
    building: bool,
}

pub fn run(_creep: &Creep, _memory: &mut BuilderMemory, _d: &SharedData) {
    // todo
}
