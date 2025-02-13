use graphql_client::{reqwest::post_graphql, GraphQLQuery};
use reqwest::Client;
use thiserror::Error;

use super::microchain::ChainState;

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error("GraphQL errors: {0:?}")]
    GraphQLError(Vec<graphql_client::Error>),
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

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "gql/schema.graphql",
    query_path = "gql/query.graphql",
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
