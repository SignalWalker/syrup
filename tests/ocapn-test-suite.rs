use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

#[test]
fn ocapn_test_suite() {
    // prepare test runner command
    let suite_dir = env::var("REXA_OCAPN_TEST_SUITE_DIR")
        .map(PathBuf::from)
        .unwrap()
        .canonicalize()
        .unwrap();
    let runner = suite_dir.join("test_runner.py");

    let python = env::var("REXA_PYTHON_PATH").unwrap_or(String::from("python"));

    let local_locator: &str = todo!();

    let mut python = Command::new(&python)
        .args([runner.to_str().unwrap(), "--verbose", local_locator])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env_clear();
}
