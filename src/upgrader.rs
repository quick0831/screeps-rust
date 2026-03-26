use screeps::Creep;
use screeps::action_error_codes::HarvestErrorCode;
use screeps::action_error_codes::UpgradeControllerErrorCode;
use screeps::find;

use crate::CreepMem;
use crate::SharedData;

pub fn run(creep: &Creep, _memory: &CreepMem, _d: &SharedData) {
    if creep.store().get_free_capacity(None) > 0 {
        let sources = creep.room().unwrap().find(find::SOURCES, None);
        if let Err(HarvestErrorCode::NotInRange) = creep.harvest(&sources[0]) {
            let _ = creep.move_to(&sources[0]);
        }
    } else {
        let controller = creep.room().unwrap().controller().unwrap();
        if let Err(UpgradeControllerErrorCode::NotInRange) = creep.upgrade_controller(&controller) {
            let _ = creep.move_to(controller);
        }
    }
}
