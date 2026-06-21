use std::process::Command;

fn run_bdk_cli(args: &str) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(format!("run --quiet -- {}", args).split_whitespace())
        .output()
        .expect("Failed to execute command");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (success, stdout, stderr)
}

#[test]
fn test_send_all_rejects_multiple_recipients() {
    let test_dir = std::env::temp_dir().join("bdk-cli-test-send-all");
    let _ = std::fs::remove_dir_all(&test_dir);
    std::fs::create_dir_all(&test_dir).unwrap();
    let datadir = test_dir.to_str().unwrap();

    // Step 1: Generate a key
    let (success, stdout, stderr) = run_bdk_cli("key generate");
    assert!(success, "Step 1 failed - Key generation: {}", stderr);

    let key_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse key output");
    let xprv = key_json["xprv"].as_str().expect("No xprv in output");

    // Step 2: Derive receive descriptor
    let (success, stdout, stderr) =
        run_bdk_cli(&format!("key derive --path m/84h/1h/0h/0 --xprv {}", xprv));
    assert!(success, "Step 2 failed - Derive receive: {}", stderr);

    let recv_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse derive output");
    let recv_xprv = recv_json["xprv"]
        .as_str()
        .expect("No xprv in derive output");
    let recv_desc = format!("wpkh({})", recv_xprv);

    // Step 3: Derive change descriptor
    let (success, stdout, stderr) =
        run_bdk_cli(&format!("key derive --path m/84h/1h/0h/1 --xprv {}", xprv));
    assert!(success, "Step 3 failed - Derive change: {}", stderr);

    let change_json: serde_json::Value =
        serde_json::from_str(&stdout).expect("Failed to parse derive output");
    let change_xprv = change_json["xprv"]
        .as_str()
        .expect("No xprv in derive output");
    let change_desc = format!("wpkh({})", change_xprv);

    // Step 4: Save wallet config
    let (success, _, stderr) = run_bdk_cli(&format!(
        "--datadir {} wallet --wallet test config --database-type sqlite -e {} -i {}",
        datadir, recv_desc, change_desc
    ));
    assert!(success, "Step 4 failed - Wallet config: {}", stderr);

    // Step 5: Try create_tx with --send_all and MULTIPLE recipients
    let addr1 = "mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn";
    let addr2 = "2MzQwSSnBHWHqSAqtTVQ6v47XtaisrJa1Vc";

    let (success, stdout, stderr) = run_bdk_cli(&format!(
        "--datadir {} wallet --wallet test create_tx --send_all \
         --to {}:0 \
         --to {}:0",
        datadir, addr1, addr2
    ));

    assert!(
        !success,
        "create_tx with --send_all and multiple recipients should fail, but it succeeded.\nstdout: {}\nstderr: {}",
        stdout, stderr
    );

    // Check that error message mentions single output
    let combined = format!("{}{}", stdout, stderr);
    assert!(
        combined.contains("drained to a single output"),
        "Expected error message about single output, got: {}",
        combined
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&test_dir);
}
