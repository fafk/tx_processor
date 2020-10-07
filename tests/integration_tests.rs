use assert_cmd::prelude::*;
use std::process::Command;

#[test]
pub fn invalid_args() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("tx_processor")?;
    cmd.arg("foobar").arg("test/file/doesnt/exist");
    cmd.assert()
        .failure();
    Ok(())
}

#[test]
pub fn correct_start() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("tx_processor")?;
    cmd.arg("test_data/001.csv");
    cmd.assert()
        .success();
    Ok(())
}

#[test]
pub fn correctly_formatted_output() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("tx_processor")?;
    cmd.arg("test_data/004.csv");

    let output = cmd.output().unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout),
               "client,available,held,total,locked\n1,-1.0,1.0,0.0,false\n");
    Ok(())
}
