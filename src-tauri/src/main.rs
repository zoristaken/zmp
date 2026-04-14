// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//TODO (zor): currently theres not good error handling
//throughout the app. It would greatly benefit if we
//bubbled errors with expressive errors and chains
//so we could have more expressive frontend error messages and easier error tracking/debugging

//TODO (zor): domain object validation?

fn main() {
    zmp_lib::run()
}
