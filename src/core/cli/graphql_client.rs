use std::{collections::HashMap, sync::Arc};

use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use once_cell::sync::Lazy;
use reqwest::Client;
use thiserror::Error;
use tokio::{fs, io, process::{Child, Command}, sync::Mutex};

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

pub type ChainId = String;
pub type CryptoHash = String;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "linera_assets/gql/chain_schema.graphql",
    query_path = "linera_assets/gql/chain_mutation.graphql",
    response_derives = "Debug"
)]
struct PublishDataBlob;

pub(crate) async fn publish_data_blob(
    port: &str,
    chain_id: ChainId,
    data: Vec<u8>,
) -> Result<CryptoHash, Error> {
    let client = reqwest_client();
    let bytes = data.into_iter().map(|b| b as i64).collect::<Vec<_>>();
    let variables = publish_data_blob::Variables { chain_id, bytes };
    let addr = format!("http://127.0.0.1:{}", port);
    Ok(request::<PublishDataBlob, _>(&client, &addr, variables)
        .await?
        .publish_data_blob)
}

pub(crate) fn linera(wallet: Option<usize>) -> Command {
    match wallet {
        Some(i) => {
            let mut cmd = Command::new("linera");
            cmd.arg(format!("-w{}", i));
            cmd
        },
        None => Command::new("linera"),
    }
}

static SERVICE_HANDLES: Lazy<Arc<Mutex<HashMap<String, Child>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Spawn a background task that runs the linera service
pub(crate) async fn linera_service(wallet: Option<usize>, port: &str) -> Result<(), Error> {
    // First, make sure any existing service on this port is killed
    linera_service_kill(port).await?;
    
    println!("Starting linera service --port {}", port);
    
    // Use spawn instead of shell backgrounding
    let child = linera(wallet)
        .arg("service")
        .arg("--port")
        .arg(port)
        .spawn()?;
    
    // Store the handle in our global map
    {
        let mut handles = SERVICE_HANDLES.lock().await;
        handles.insert(port.to_string(), child);
    }
    
    // Give the service some time to start up
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Optional: Verify the service is actually running
    match verify_service_running(port).await {
        Ok(_) => {
            println!("Service on port {} successfully started", port);
            Ok(())
        },
        Err(e) => {
            // Clean up if verification fails
            linera_service_kill(port).await?;
            Err(e)
        }
    }
}

/// Kill the linera service running on the specified port
pub(crate) async fn linera_service_kill(port: &str) -> Result<(), Error> {
    // First try to kill using our stored handle
    {
        let mut handles = SERVICE_HANDLES.lock().await;
        if let Some(mut child) = handles.remove(port) {
            match child.kill().await {
                Ok(_) => {
                    println!("Service on port {} terminated via handle", port);
                    return Ok(());
                },
                Err(e) => {
                    println!("Failed to kill service via handle: {}", e);
                    // Fall through to pkill approach
                }
            }
        }
    }
    
    // Fallback: Use pkill as a backup method
    println!("Attempting to kill service on port {} via pkill", port);
    let output = Command::new("pkill")
        .arg("-f")
        .arg(format!("linera.*service.*--port {}($|[^0-9])", port))
        .output()
        .await?;
        
    if !output.status.success() {
        // Not finding a process is fine - it might not be running
        if !output.stderr.is_empty() {
            println!("pkill stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    // Give a moment for the process to terminate
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(())
}

/// Verify that the service is actually running and responding
async fn verify_service_running(port: &str) -> Result<(), Error> {
    use reqwest::Client;
    use std::time::Duration;
    
    let client = Client::new();
    let url = format!("http://localhost:{}", port);
    
    // Try a few times with increasing delays
    for attempt in 1..=5 {
        match client.get(&url)
            .timeout(Duration::from_secs(1))
            .send()
            .await 
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                println!("Service verification attempt {}: {}", attempt, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(attempt)).await;
            }
        }
    }
    
    Err(Error::String("Service failed to start after multiple attempts".to_string()))
}

pub(crate) async fn create_lurk_microchain(
    wallet: Option<usize>,
    port: &str,
    genesis: &[u8],
    contract: &str,
    service: &str,
) -> Result<(String, String), Error> {
    use tempfile::tempdir;

    // Create a temporary file to store the genesis data
    let temp_dir = tempdir()?;
    let genesis_path = temp_dir.path().join("genesis_state");
    fs::write(&genesis_path, &genesis).await?;

    let output = Command::new("linera")
        .arg("--with-wallet")
        .arg("0")
        .arg("publish-data-blob")
        .arg(genesis_path)
        .output()
        .await?;

    if !output.status.success() {
        panic!(
            "publish-data-blob command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Extract the GENESIS_BLOB_ID from the output
    let genesis_blob_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let output = Command::new("linera")
    .arg("--with-wallet")
    .arg("0")
        .arg("--wait-for-outgoing-messages")
        .arg("publish-and-create")
        .arg(contract)
        .arg(service)
        .output()
        .await?;

    if !output.status.success() {
        panic!(
            "publish-and-create command failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Extract the APP_ID from the output
    let app_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    linera_service(wallet, port).await?;

    Ok((genesis_blob_id, app_id))
}

async fn execute_start_mutation(
    gql_url: &str,
    owner: &str,
    other_owner: &str,
    genesis_blob_id: &str,
) -> Result<(), Error> {
    let client = reqwest::Client::new();
    let mutation = format!(
        r#"mutation {{
            start(
                accounts: [
                    "{owner}",
                    "{other_owner}"
                ],
                chainState: "{genesis_blob_id}"
            )
        }}"#
    );

    println!("querying: {}", mutation);

    client
        .post(gql_url)
        .body(
            serde_json::json!({
                "query": mutation
            })
            .to_string(),
        )
        .header("Content-Type", "application/json")
        .send()
        .await?;

    Ok(())
}

async fn execute_chains_query(gql_url: &str, owner: &str) -> Result<(String, String), Error> {
    let client = reqwest::Client::new();
    let query = format!(
        r#"query {{
            chains {{
                entry(key: "{owner}") {{
                    value {{
                        messageId chainId
                    }}
                }}
            }}
        }}"#
    );

    println!("querying: {}", query);

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
    /*
    {
      "data": {
        "chains": {
          "entry": {
            "value": [
              {
                "messageId": "779321493b4e76478132c39d5b4efbf09ca8b1db53eb99a07d3482315c70ed87160000000000000000000000",
                "chainId": "6c8e32aff3999c2a143e49474ac870147540753244502e5a20f945a53c458957"
              }
            ]
          }
        }
      }
    }

    */
    let result: serde_json::Value = response.json().await?;
    println!("result: {}", result);
    let chain_id = result["data"]["chains"]["entry"]["value"][0]["chainId"]
        .as_str()
        .ok_or(Error::String("Could not extract chainId".to_string()))?
        .to_string();

    let message_id = result["data"]["chains"]["entry"]["value"][0]["messageId"]
        .as_str()
        .ok_or(Error::String("Could not extract messageId".to_string()))?
        .to_string();

    Ok((chain_id, message_id))
}

async fn setup_microchain(wallet: Option<usize>, owner: &str, message_id: &str, port: &str) -> Result<(), Error> {
    linera_service_kill(port).await?;

    linera(wallet)
        .args(&["assign", "--owner", owner, "--message-id", message_id])
        .status()
        .await?;

    linera_service(wallet, port).await?;

    Ok(())
}

pub async fn microchain_start(
    wallet: Option<usize>,
    port: &str,
    chain: &str,
    owner: &str,
    other_owner: &str,
    genesis: &[u8],
    contract: &str,
    service: &str,
) -> Result<String, Error> {
    // linera_service_kill().await?;

    println!("Creating application...");
    let (genesis_blob_id, app_id) =
        create_lurk_microchain(wallet, port, genesis, contract, service).await?;

    let gql_url = format!(
        "http://localhost:{}/chains/{}/applications/{}",
        port, chain, app_id
    );

    println!("Starting microchain...");
    execute_start_mutation(&gql_url, owner, other_owner, &genesis_blob_id).await?;
    println!("Executing queries...");
    let (microchain_id, message_id) = execute_chains_query(&gql_url, &owner).await?;
    println!("Finishing...");
    setup_microchain(wallet ,&owner, &message_id, &port).await?;

    println!(
        "Microchain started with chainId: {}, messageId: {}",
        microchain_id, message_id
    );

    Ok(app_id)
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "linera_assets/gql/schema.graphql",
    query_path = "linera_assets/gql/query.graphql",
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

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "linera_assets/gql/schema.graphql",
    query_path = "linera_assets/gql/mutation.graphql",
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
