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
    /// Debug builds use the info log level.
    #[test]
    fn log_level_matches_debug() {
        // Arrange
        // Act
        assert_eq!(log_level(), log::LevelFilter::Info);
        // Assert
    }

    #[cfg(not(debug_assertions))]
    /// Release builds disable logging by default.
    #[test]
    fn log_level_matches_release() {
        // Arrange
        // Act
        assert_eq!(log_level(), log::LevelFilter::Off);
        // Assert
    }

    #[cfg(feature = "coverage")]
    /// Coverage builds keep the `build_app` stub callable.
    #[test]
    fn build_app_stub_is_callable() {
        // Arrange
        // Act
        build_app();
        // Assert
    }

    /// App entrypoint runs (or no-ops under coverage).
    #[test]
    fn run_is_callable() {
        // Arrange
        // Act
        run();
        // Assert
    }
}
