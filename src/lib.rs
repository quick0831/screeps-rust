use screeps::{Part, find, game};

#[unsafe(export_name = "loop")]
pub extern "C" fn func_loop() {
    let creeps = game::creeps();
    let spawn = game::spawns().values().next().unwrap();

    for creep in creeps.values() {
        let sources = creep.room().unwrap().find(find::SOURCES, None);
        let _ = creep.move_to(&sources[0]);
    }

    if creeps.keys().count() == 0 {
        let body = vec![Part::Move, Part::Work, Part::Carry];
        let _ = spawn.spawn_creep(&body, "Test");
    }
}
