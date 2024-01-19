use rexa::locator::NodeLocator;
use std::{
    collections::HashMap,
    env,
    path::PathBuf,
    process::{Command, ExitStatus, Stdio},
};
use syrup::ser::to_bytes;

#[test]
fn ocapn_test_suite() -> std::io::Result<()> {
    // prepare test runner command
    let suite_dir = env::var("REXA_OCAPN_TEST_SUITE_DIR")
        .map(PathBuf::from)
        .unwrap()
        .canonicalize()
        .unwrap();
    let runner = suite_dir.join("test_runner.py");

    let python = env::var("REXA_PYTHON_PATH").unwrap_or(String::from("python"));

    let local_locator: NodeLocator<String, String> = NodeLocator {
        designator: "todo".to_owned(),
        transport: "onion".to_owned(),
        hints: HashMap::new(),
    };
    let local_locator_uri = local_locator.to_uri();

    let mut python = Command::new(&python);
    python
        .args([runner.to_str().unwrap(), "--verbose", &local_locator_uri])
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut proc = python.spawn().unwrap();
    proc.wait().map(|_| ())
}
