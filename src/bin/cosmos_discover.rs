use anyhow::{Context, Result};
use azure_core::credentials::Secret;
use azure_data_cosmos::{
    feed::FeedScope, options::Region, AccountEndpoint, AccountReference, CosmosClient,
    RoutingStrategy,
};
use futures::TryStreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().context("Failed to load .env")?;

    let endpoint =
        std::env::var("COSMOS_ENDPOINT").context("COSMOS_ENDPOINT is missing from .env")?;

    let key = std::env::var("COSMOS_KEY").context("COSMOS_KEY is missing from .env")?;

    let endpoint: AccountEndpoint = endpoint.parse().context("Invalid COSMOS_ENDPOINT")?;

    let account = AccountReference::with_authentication_key(endpoint, Secret::from(key));

    let client = CosmosClient::builder()
        .build(account, RoutingStrategy::ProximityTo(Region::WEST_EUROPE))
        .await
        .context("Failed to create Cosmos client")?;

    println!("Connected to Cosmos DB");
    println!();

    // ---------------------------------------------------------
    // DATABASES
    // ---------------------------------------------------------

    println!("Databases:");

    let mut databases = client
        .query_databases("SELECT * FROM dbs", None)
        .await
        .context("Failed to query Cosmos databases")?;

    let mut database_count = 0;

    while let Some(database) = databases.try_next().await? {
        println!("  * {database:#?}");
        database_count += 1;
    }

    println!("Found {database_count} database(s)");

    println!();

    // ---------------------------------------------------------
    // GOLDEN RECORD CONTAINERS
    // ---------------------------------------------------------

    println!("Containers in GoldenRecord:");

    let database = client.database_client("GoldenRecord");

    let mut containers = database
        .query_containers("SELECT * FROM c", None)
        .await
        .context("Failed to query GoldenRecord containers")?;

    let mut container_count = 0;

    while let Some(container) = containers.try_next().await? {
        println!("  * {container:#?}");
        container_count += 1;
    }

    println!("Found {container_count} container(s)");

    println!();

    // ---------------------------------------------------------
    // RAW ERP DATA
    // ---------------------------------------------------------

    println!("Reading one document from Record:");

    let record_container = database
        .container_client("Record", None)
        .await
        .context("Failed to create Record container client")?;

    let query = r#"
    SELECT TOP 1 *
    FROM c
    WHERE IS_DEFINED(c.PimData.Data)
      AND NOT IS_NULL(c.PimData.Data)
      AND IS_DEFINED(c.ErpData.Data)
      AND NOT IS_NULL(c.ErpData.Data)
"#;

    let mut items = record_container
        .query_items::<serde_json::Value>(query, FeedScope::full_container(), None)
        .await
        .context("Failed to query Record")?;

    match items.try_next().await? {
        Some(item) => {
            println!();
            println!("Record document:");
            println!(
                "{}",
                serde_json::to_string_pretty(&item).context("Failed to format Record document",)?
            );
        }

        None => {
            println!("Record contains no documents.");
        }
    }

    Ok(())
}
