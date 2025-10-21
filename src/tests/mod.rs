#[tokio::test]
async fn test_with_pubkey_echo() -> anyhow::Result<()> {
    use super::*;
    use crate::ssh::SSHClient;
    use std::path::PathBuf;
    let host = "10.210.126.58";
    let port = 22;
    let user = "harkerhand";
    let privkey_path = PathBuf::from("C:\\Users\\harkerhand\\.ssh\\id_ed25519");
    let client = SSHClient::with_key(privkey_path, user, None, (host, port)).await;
    assert!(client.is_ok());
    let out = client?.client.execute("echo hello").await?;
    assert_eq!(out.exit_status, 0);
    assert_eq!(out.stdout.trim(), "hello");

    Ok(())
}
