use screeps::CostMatrix;
use screeps::Creep;
use screeps::Position;
use screeps::action_error_codes::CreepMoveByPathErrorCode;
use screeps::game;
use screeps::pathfinder::MultiRoomCostResult;
use screeps::pathfinder::SearchOptions;
use screeps::pathfinder::search;
use screeps::prelude::*;

const COST_UNWALKABLE: u8 = 255;

pub fn path_away_from(
    creep: &Creep,
    target: Position,
    range: u32,
) -> Result<(), CreepMoveByPathErrorCode> {
    // TODO: cache the result when queried on the same tick
    let callback = |room| {
        let cost_matrix = CostMatrix::new();
        for c in game::creeps().values() {
            let pos = c.pos();
            if pos.room_name() == room {
                let (x, y) = pos.coords();
                cost_matrix.set(x, y, COST_UNWALKABLE);
            }
        }
        MultiRoomCostResult::CostMatrix(cost_matrix)
    };

    let options = SearchOptions::new(callback)
        .flee(true)
        .max_rooms(1)
        .max_ops(100);
    let result = search(creep.pos(), target, range, Some(options));
    creep.move_by_path(&result.opaque_path())
}
