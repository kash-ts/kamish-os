pub use kamish_core as core;
pub use kamish_ipc as ipc;
pub use kamish_web as web;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metapackage_crates_accessible() {
        assert_eq!(core::config::Config::from_env().env, "development");
        assert_eq!(ipc::DEFAULT_SOCKET_PATH, "/run/kamish/core.sock");
    }
}
