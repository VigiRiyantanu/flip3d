#![windows_subsystem = "windows"]

// Original Windhawk mod by caliberda
// Ported to standalone Rust by Vigi Riyantanu

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let debug_mode = args.iter().any(|a| a == "--debug" || a == "-d");
    settings_ui::run_settings_ui(debug_mode);
}
