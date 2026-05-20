// Supporting subdirectory module for file_modules.ox — loaded via `mod subpkg;`
// This file is at subpkg/mod.ox and must be valid standalone.

pub mod helper {
    pub fn greet() -> String {
        "hello from subpkg".to_string()
    }
}
