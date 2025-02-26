use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use reqwest::Client;
use thiserror::Error;
use tokio::{fs, io, process::Command};

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

pub(crate) async fn microchain_start(genesis: &[u8], contract: &str, service: &str) -> Result<String, Error> {
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

    let json_arg = format!("{{ \"chain_state\": \"{}\" }}", genesis_blob_id);
    let output = Command::new("linera")
        .arg("--with-wallet")
        .arg("0")
        .arg("publish-and-create")
        .arg(contract)
        .arg(service) // Handle the brace expansion from the shell command
        .arg("--json-argument")
        .arg(json_arg)
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
    Ok(app_id)
}

/// Spawn a background task that runs the linera service
pub(crate) async fn linera_service(port: String) -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn(async move {
        let result = Command::new("linera")
            .arg("--with-wallet")
            .arg("0")
            .arg("service")
            .arg("--port")
            .arg(port)
            .spawn()
            .expect("Failed to start linera service")
            .wait()
            .await;
            
        if let Err(e) = result {
            eprintln!("Linera service exited with error: {}", e);
        }
    });
    
    // Give the service a moment to start up
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    handle
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
