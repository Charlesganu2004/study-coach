//! Credential configuration checks use fake process values and make no API calls.
use std::process::Command;

#[test]
fn help_documents_standard_dotenv_and_precedence() {
    let output = Command::new(env!("CARGO_BIN_EXE_study-coach"))
        .arg("help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(".env and .env.local"));
    assert!(text.contains("process values take precedence"));
}

#[test]
fn doctor_uses_process_credentials_without_exposing_them() {
    let output = Command::new(env!("CARGO_BIN_EXE_study-coach"))
        .arg("doctor")
        .env("GROQ_API_KEY", "FAKE_GROQ_PROCESS_CREDENTIAL")
        .env("NVIDIA_API_KEY", "FAKE_NVIDIA_PROCESS_CREDENTIAL")
        .output()
        .unwrap();
    assert!(output.status.success(), "doctor prerequisite check failed");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(text.contains("GROQ_API_KEY configured: true"));
    assert!(text.contains("NVIDIA_API_KEY configured: true"));
    assert!(text.contains(".env.local, then .env"));
    assert!(!text.contains("FAKE_GROQ_PROCESS_CREDENTIAL"));
    assert!(!text.contains("FAKE_NVIDIA_PROCESS_CREDENTIAL"));
}
