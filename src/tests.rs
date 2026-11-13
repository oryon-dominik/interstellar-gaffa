#[cfg(test)]
mod test {
    use crate::constants::*;
    use crate::types::{ProcessError, ProcessStatus};

    #[test]
    fn test_process_status_display() {
        // ProcessStatus Display includes colors, but in tests they might be disabled
        let running = ProcessStatus::Running.to_string();
        let stopped = ProcessStatus::Stopped.to_string();
        let restarting = ProcessStatus::Restarting.to_string();

        // Check that the status strings contain the expected text
        assert!(running.contains("RUNNING"));
        assert!(stopped.contains("STOPPED"));
        assert!(restarting.contains("RESTARTING"));
    }

    #[test]
    fn test_process_error_messages() {
        let err = ProcessError::ProcessNotFound {
            name: "test".to_string(),
        };
        assert_eq!(err.to_string(), "Process 'test' not found");

        let err = ProcessError::ProcessAlreadyRunning {
            name: "web".to_string(),
        };
        assert_eq!(err.to_string(), "Process 'web' is already running");

        let err = ProcessError::EmptyCommand {
            name: "worker".to_string(),
        };
        assert_eq!(err.to_string(), "Empty command for process 'worker'");
    }

    #[test]
    fn test_parse_env_file() {
        use std::collections::HashMap;
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();

        // Create test env file
        let env_content = "KEY1=value1\nKEY2=value2\n# Comment\n\nKEY3=value3";
        let path = std::env::temp_dir().join(format!("gaffa_test_env_file_{}", std::process::id()));
        let path = path.to_string_lossy().into_owned();
        std::fs::write(&path, env_content).unwrap();

        let mut env_vars = HashMap::new();

        rt.block_on(async {
            crate::parse_env_file(&path, &mut env_vars).await.unwrap();
        });

        assert_eq!(env_vars.get("KEY1"), Some(&"value1".to_string()));
        assert_eq!(env_vars.get("KEY2"), Some(&"value2".to_string()));
        assert_eq!(env_vars.get("KEY3"), Some(&"value3".to_string()));
        assert_eq!(env_vars.len(), 3);

        // Cleanup
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_parse_env_file_edge_cases() {
        use std::collections::HashMap;
        use tokio::runtime::Runtime;

        let rt = Runtime::new().unwrap();

        // Test with spaces and quotes
        let env_content = "KEY1 = value with spaces\nKEY2=\"quoted value\"\nKEY3=";
        let path = std::env::temp_dir().join(format!("gaffa_test_env_edge_{}", std::process::id()));
        let path = path.to_string_lossy().into_owned();
        std::fs::write(&path, env_content).unwrap();

        let mut env_vars = HashMap::new();

        rt.block_on(async {
            crate::parse_env_file(&path, &mut env_vars).await.unwrap();
        });

        assert_eq!(env_vars.get("KEY1"), Some(&"value with spaces".to_string()));
        assert_eq!(env_vars.get("KEY2"), Some(&"\"quoted value\"".to_string()));
        assert_eq!(env_vars.get("KEY3"), Some(&"".to_string()));

        // Cleanup
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_force_kill_window_fits_the_graceful_window() {
        assert!(PROCESS_WAIT_TIMEOUT < GRACEFUL_SHUTDOWN_TIMEOUT);
    }
}
