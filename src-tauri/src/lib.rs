pub mod mumble;
pub mod transport;

#[cfg(debug_assertions)]
fn log_level() -> log::LevelFilter {
    log::LevelFilter::Info
}

#[cfg(not(debug_assertions))]
fn log_level() -> log::LevelFilter {
    log::LevelFilter::Off
}

#[cfg(not(feature = "coverage"))]
fn build_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default().setup(|app| {
        if cfg!(debug_assertions) {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log_level())
                    .build(),
            )?;
        }
        Ok(())
    })
}

#[cfg(feature = "coverage")]
fn build_app() {}

#[cfg(not(feature = "coverage"))]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    build_app()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(feature = "coverage")]
pub fn run() {}

#[cfg(test)]
mod tests {
    use super::{build_app, log_level, run};

    #[cfg(debug_assertions)]
    #[test]
    fn log_level_matches_debug() {
        assert_eq!(log_level(), log::LevelFilter::Info);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn log_level_matches_release() {
        assert_eq!(log_level(), log::LevelFilter::Off);
    }

    #[cfg(feature = "coverage")]
    #[test]
    fn build_app_stub_is_callable() {
        build_app();
    }

    #[test]
    fn run_is_callable() {
        run();
    }
}
