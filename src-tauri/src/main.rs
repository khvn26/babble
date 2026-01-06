// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(feature = "coverage"))]
fn app_main() {
    app_lib::run();
}

#[cfg(feature = "coverage")]
fn app_main() {}

fn main() {
    app_main();
}

#[cfg(test)]
mod tests {
    use super::app_main;

    /// Calling the app entry helper does not panic.
    #[test]
    fn app_main_is_callable() {
        // Arrange
        // Act
        app_main();
        // Assert
    }

    /// Calling the top-level main does not panic.
    #[test]
    fn main_is_callable() {
        // Arrange
        // Act
        super::main();
        // Assert
    }
}
