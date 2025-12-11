use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_check_command_with_valid_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        {{
             "tasks": [
             {{
                  "name": "valid_task",
                  "cron_schedule": "* * * * * *",
                  "command": "echo",
                  "args": ["ok"]
              }}
             ]
         }}"#
    )
    .unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_chronsync"));
    cmd.arg("check")
        .arg("--config-path")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration check passed"));
}

#[test]
fn test_check_command_with_invalid_file() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        {{
             "tasks": [
             {{
                  "name": "invalid_task",
                  "cron_schedule": "INVALID_CRON",
                  "command": "echo"
              }}
             ]
         }}"#
    )
    .unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_chronsync"));
    cmd.arg("check")
        .arg("--config-path")
        .arg(file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("Validation failed"));
}

#[test]
fn test_help_command() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_chronsync"));
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("daemon"));
}

#[test]
fn test_exec_command_valid_task() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        {{
             "tasks": [
             {{
                  "name": "hello_exec",
                  "cron_schedule": "* * * * * *",
                  "command": "echo",
                  "args": ["MANUAL_EXEC_SUCCESS"],
                  "timeout": 5
              }}
             ]
         }}"#
    )
    .unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_chronsync"));
    cmd.arg("exec")
        .arg("hello_exec")
        .arg("--config-path")
        .arg(file.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("MANUAL_EXEC_SUCCESS"))
        .stdout(predicate::str::contains("Manually executing task"));
}

#[test]
fn test_exec_command_unknown_task() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        {{
             "tasks": [
             {{
                  "name": "existing_task",
                  "cron_schedule": "* * * * * *",
                  "command": "echo"
              }}
             ]
         }}"#
    )
    .unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_chronsync"));
    cmd.arg("exec")
        .arg("ghost_task")
        .arg("--config-path")
        .arg(file.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("Task 'ghost_task' not found"));
}
