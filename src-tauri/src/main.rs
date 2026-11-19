// Sem console extra no Windows em release. NÃO REMOVER.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if lobby_companion_lib::cli(&args) {
        return;
    }
    lobby_companion_lib::run()
}
