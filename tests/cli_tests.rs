use std::path::PathBuf;
use std::process::Command;

#[macro_use]
extern crate assert_json_diff;

fn get_resource_path(file_name: &str) -> String {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("resources/test");
    d.push(file_name);
    return d.to_str().unwrap().to_owned();
}

#[test]
fn chrome_tracing_converter() {
    let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    d.push("resources/test/test_trace.htdump");

    let mut command = Command::new("cargo");
    command.args(&[
        "run",
        "--",
        "--source",
        &get_resource_path("test_trace.htdump"),
        "--stdout",
    ]);

    let mut output = String::from_utf8(command.output().unwrap().stdout).unwrap();

    // converter doesn't close the JSON correctly - Chrome Tracing tool works fine with that,
    // but serde loader can't load it, so we are fixing it manually here.
    output.remove(output.len() - 1);
    output.push(']');

    let value: serde_json::Value = serde_json::from_str(&output).unwrap();
    let expected: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(get_resource_path("chrome_tracing_test_trace.json")).unwrap(),
    )
    .unwrap();

    assert_json_eq!(value, expected);
}
