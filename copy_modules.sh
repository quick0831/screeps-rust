#SCRIPT_PATH=test
SCRIPT_PATH=~/.config/Screeps/scripts/screeps.com/rust
set -e
cargo b -r
wasm-bindgen target/wasm32-unknown-unknown/release/screeps_rust.wasm --target no-modules --no-typescript --out-name bot --out-dir $SCRIPT_PATH

echo "module.exports.wbg = wasm_bindgen;" >>$SCRIPT_PATH/bot.js
cp main.js $SCRIPT_PATH/
