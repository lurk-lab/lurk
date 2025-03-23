use crate::core::cli::{
    config::{set_config_if_unset, Config},
    repl::Repl,
};

#[tokio::test]
async fn test_meta_commands() {
    set_config_if_unset(Config::default());
    let mut repl = Repl::new_native(false, false, None);
    assert!(repl
        .load_file("src/core/cli/tests/first.lurk".into(), false)
        .await
        .is_ok());
    let mut repl = Repl::new_native(false, false, None);
    assert!(repl
        .load_file("src/core/cli/tests/second.lurk".into(), false)
        .await
        .is_ok());
    std::fs::remove_file("repl-test-two").unwrap();
}

#[ignore]
#[tokio::test]
async fn test_meta_commands_with_proofs() {
    set_config_if_unset(Config::default());
    let mut repl = Repl::new_native(false, false, None);
    assert!(repl
        .load_file("src/core/cli/tests/prove.lurk".into(), false)
        .await
        .is_ok());
    let mut repl = Repl::new_native(false, false, None);
    assert!(repl
        .load_file("src/core/cli/tests/verify.lurk".into(), false)
        .await
        .is_ok());
    std::fs::remove_file("repl-test-protocol-proof").unwrap();
    std::fs::remove_file("repl-test-protocol").unwrap();
}

#[tokio::test]
async fn test_lib() {
    set_config_if_unset(Config::default());
    let mut repl = Repl::new_native(false, false, None);
    assert!(repl.load_file("lib/tests.lurk".into(), false).await.is_ok());
}

#[ignore]
#[tokio::test]
async fn test_demo_files() {
    set_config_if_unset(Config::default());
    let demo_files = [
        "demo/simple.lurk",
        "demo/functional-commitment.lurk",
        "demo/chained-functional-commitment.lurk",
        "demo/protocol.lurk",
        "demo/bank.lurk",
        "demo/mastermind.lurk",
        "demo/mini-mastermind.lurk",
        "demo/microbank.lurk",
    ];
    for file in demo_files {
        let mut repl = Repl::new_native(false, false, None);
        assert!(repl.load_file(file.into(), false).await.is_ok());
    }
    std::fs::remove_file("protocol-proof").unwrap();
}
