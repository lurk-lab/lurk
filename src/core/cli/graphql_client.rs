use std::{collections::HashMap, process::Stdio, sync::Arc};

use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use once_cell::sync::Lazy;
use reqwest::Client;
use thiserror::Error;
use tokio::{
    io,
    process::{Child, Command},
    sync::Mutex,
};

use super::microchain::ChainState;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("GraphQL errors: {0:?}")]
    GraphQLError(Vec<graphql_client::Error>),
    #[error(transparent)]
    IoError(#[from] io::Error),
    #[error("{0:?}")]
    String(String),
}

impl From<Option<Vec<graphql_client::Error>>> for Error {
    fn from(val: Option<Vec<graphql_client::Error>>) -> Self {
        Self::GraphQLError(val.unwrap_or_default())
    }
}

pub async fn request<T, V>(
    client: &Client,
    url: &str,
    variables: V,
) -> Result<T::ResponseData, Error>
where
    T: GraphQLQuery<Variables = V> + Send + Unpin + 'static,
    V: Send + Unpin,
{
    let response = post_graphql::<T, _>(client, url, variables).await?;
    match response.data {
        None => Err(response.errors.into()),
        Some(data) => Ok(data),
    }
}

pub(crate) fn reqwest_client() -> reqwest::Client {
    reqwest::ClientBuilder::new().build().unwrap()
}

pub(crate) fn linera(wallet: Option<usize>) -> Command {
    match wallet {
        Some(i) => {
            let mut cmd = Command::new("linera");
            cmd.arg(format!("-w{}", i));
            cmd
        }
        None => Command::new("linera"),
    }
}

static SERVICE_HANDLES: Lazy<Arc<Mutex<HashMap<String, Child>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Spawn a background task that runs the linera service
pub(crate) async fn linera_service(
    wallet: Option<usize>,
    port: &str,
    quiet: bool,
) -> Result<(), Error> {
    // println!("Starting linera service --port {}", port);
    let mut command = linera(wallet);
    command.arg("service").arg("--port").arg(port);

    if quiet {
        command.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let child = command.spawn()?;

    let mut handles = SERVICE_HANDLES.lock().await;
    handles.insert(port.to_string(), child);

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}

/// Kill the linera service running on the specified port
pub(crate) async fn linera_service_kill(port: &str) -> Result<(), Error> {
    let mut handles = SERVICE_HANDLES.lock().await;

    if let Some(mut child) = handles.remove(port) {
        match child.kill().await {
            Ok(_) => {
                // println!("Service on port {} terminated via handle", port);
                return Ok(());
            }
            Err(e) => {
                println!("No port found: {}", e);
            }
        }
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    Ok(())
}

pub(crate) async fn post_query(gql_url: &str, query: &str) -> Result<serde_json::Value, Error> {
    println!("==============================================================");
    println!("Posting querying at {gql_url}... (query elided)");
    // println!("{}", query);

    let client = reqwest::Client::new();
    let response = client
        .post(gql_url)
        .body(
            serde_json::json!({
                "query": query
            })
            .to_string(),
        )
        .header("Content-Type", "application/json")
        .send()
        .await?;
    let result: serde_json::Value = response.json().await?;
    let result = result["data"].clone();

    println!("Result:");
    println!("{}", result);
    println!("==============================================================\n");

    Ok(result)
}

pub(crate) async fn publish_data_blob(
    gql_url: &str,
    chain_id: &str,
    bytes: &[u8],
) -> Result<String, Error> {
    let query = format!(
        r#"mutation {{
            publishDataBlob(chainId: "{chain_id}", bytes: {bytes:?} )
        }}"#
    );

    let result = post_query(gql_url, &query).await?;
    let blob_id = result["publishDataBlob"].as_str().unwrap().to_string();

    Ok(blob_id)
}

pub(crate) async fn linera_create(
    wallet: Option<usize>,
    chain_id: &str,
    contract: &str,
    service: &str,
) -> Result<String, Error> {
    // println!("Creating application {}...", contract);
    let output = linera(wallet)
        .arg("--wait-for-outgoing-messages")
        .arg("publish-and-create")
        .arg(contract)
        .arg(service)
        .arg(chain_id)
        .output()
        .await?;

    if !output.status.success() {
        panic!(
            "publish-and-create command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let app_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(app_id)
}

async fn execute_start_mutation(
    gql_url: &str,
    owner: &str,
    genesis_blob_id: &str,
) -> Result<(), Error> {
    let mutation = format!(
        r#"mutation {{
            start(
                owner: "{owner}",
                chainState: "{genesis_blob_id}"
            )
        }}"#
    );

    let _ = post_query(gql_url, &mutation).await?;
    Ok(())
}

async fn execute_ready_query(gql_url: &str) -> Result<(String, String), Error> {
    let query = r#"query {
            ready {
                messageId chainId
            }
        }"#
    .to_string();

    let result = post_query(gql_url, &query).await?;

    let chain_id = result["ready"]["chainId"].as_str().unwrap().to_string();
    let message_id = result["ready"]["messageId"].as_str().unwrap().to_string();

    Ok((chain_id, message_id))
}

async fn setup_microchain(
    wallet: Option<usize>,
    owner: &str,
    message_id: &str,
) -> Result<(), Error> {
    linera(wallet)
        .args(["assign", "--owner", owner, "--message-id", message_id])
        .status()
        .await?;

    Ok(())
}

pub async fn microchain_start(
    port: &str,
    chain: &str,
    app: &str,
    owner: &str,
    genesis: &[u8],
) -> Result<(), Error> {
    // println!("Starting microchain...");

    let gql_url = format!("http://127.0.0.1:{port}");
    let app_gql_url = format!("http://127.0.0.1:{port}/chains/{chain}/applications/{app}");

    let genesis_blob_id = publish_data_blob(&gql_url, chain, genesis).await?;
    execute_start_mutation(&app_gql_url, owner, &genesis_blob_id).await?;

    Ok(())
}

pub async fn setup_spawn(
    wallet: Option<usize>,
    port: &str,
    chain: &str,
    app: &str,
    owner: &str,
) -> Result<String, Error> {
    let app_gql_url = format!("http://127.0.0.1:{port}/chains/{chain}/applications/{app}");
    let (chain_id, message_id) = execute_ready_query(&app_gql_url).await?;

    linera_service_kill(port).await?;
    setup_microchain(wallet, owner, &message_id).await?;
    linera_service(wallet, port, false).await?;

    Ok(chain_id)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "linera-assets/gql/schema.graphql",
    query_path = "linera-assets/gql/query.graphql",
    response_derives = "Debug"
)]
struct MicrochainGetState;

pub(crate) async fn microchain_get_state(addr_str: &str) -> Result<ChainState, Error> {
    let client = reqwest_client();
    let variables = microchain_get_state::Variables {};
    let chain_state = request::<MicrochainGetState, _>(&client, addr_str, variables)
        .await?
        .chain_state;
    let bytes = chain_state.iter().map(|x| *x as u8).collect::<Vec<_>>();
    let chain_state: ChainState =
        bincode::deserialize(&bytes).expect("couldn't deserialize chain state");
    Ok(chain_state)
}

type CryptoHash = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "linera-assets/gql/schema.graphql",
    query_path = "linera-assets/gql/mutation.graphql",
    response_derives = "Debug"
)]
struct MicrochainTransition;

pub(crate) async fn microchain_transition(id_str: &str, hash: CryptoHash) -> Result<String, Error> {
    let client = reqwest_client();
    let query = format!("mutation {{ transition( chainProof: \"{}\") }}", hash);
    let response = client
        .post(id_str)
        .json(&serde_json::json!({ "query": query }))
        .send()
        .await
        .unwrap();

    let value: serde_json::Value = response.json().await.expect("invalid JSON");
    if let Some(errors) = value.get("error") {
        // println!("error: {}", errors);
        Err(Error::String(errors.to_string()))
    } else {
        // println!("data: {}", value["data"]);
        Ok("".to_string())
    }
}
