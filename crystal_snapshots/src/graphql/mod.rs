/// Collection of graphql entities
mod entities;
use crate::S3Bucket;
use async_graphql::{
    dataloader::{DataLoader, Loader},
    ComplexObject, Context, EmptyMutation, EmptySubscription, Object, Schema, SchemaBuilder,
};
use aws_sdk_s3::{presigning::PresigningConfig, Client};
use entities::DataCollection;
use models::data_collection;

use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::time::Duration;
use tracing::Span;

/// The GraphQL schema exposed by the service
pub type RootSchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// router handler extension
pub trait AddDataLoadersExt {
    /// Adds dataloader to graphql request
    fn add_data_loaders(
        self,
        database: DatabaseConnection,
        s3_client: Client,
        s3_bucket: S3Bucket,
    ) -> Self;
}

impl AddDataLoadersExt for async_graphql::Request {
    fn add_data_loaders(
        self,
        database: DatabaseConnection,
        s3_client: Client,
        s3_bucket: S3Bucket,
    ) -> Self {
        self.data(DataLoader::new(
            CrystalSnapshotDataLoader::new(database.clone(), s3_client.clone(), s3_bucket.clone()),
            tokio::spawn,
        ))
    }
}

/// A schema builder for the service
pub fn root_schema_builder() -> SchemaBuilder<Query, EmptyMutation, EmptySubscription> {
    Schema::build(Query, EmptyMutation, EmptySubscription).enable_federation()
}

/// The root query of the service
#[derive(Debug, Clone, Default)]
pub struct Query;

/// DataLoader for Crystal Snapshots data
#[allow(clippy::missing_docs_in_private_items)]
pub struct CrystalSnapshotDataLoader {
    database: DatabaseConnection,
    parent_span: Span,
    s3_client: Client,
    s3_bucket: S3Bucket,
}

#[allow(clippy::missing_docs_in_private_items)]
impl CrystalSnapshotDataLoader {
    fn new(database: DatabaseConnection, s3_client: Client, s3_bucket: S3Bucket) -> Self {
        Self {
            database,
            parent_span: Span::current(),
            s3_client,
            s3_bucket,
        }
    }
}

impl Loader<u32> for CrystalSnapshotDataLoader {
    type Value = Vec<String>;
    type Error = async_graphql::Error;

    async fn load(&self, keys: &[u32]) -> Result<HashMap<u32, Self::Value>, Self::Error> {
        let span = tracing::info_span!(parent: &self.parent_span, "load_crystal_snapshot");
        let _span = span.enter();
        let mut results = HashMap::new();
        let keys_vec: Vec<u32> = keys.to_vec();
        let records = data_collection::Entity::find()
            .filter(data_collection::Column::DataCollectionId.is_in(keys_vec))
            .all(&self.database)
            .await?;

        for record in records {
            let id = record.data_collection_id;
            let snaps = vec![
                record.xtal_snapshot_full_path1,
                record.xtal_snapshot_full_path2,
                record.xtal_snapshot_full_path3,
                record.xtal_snapshot_full_path4,
            ];

            for snap in snaps.into_iter().flatten() {
                let object_uri = self
                    .s3_client
                    .get_object()
                    .bucket(self.s3_bucket.clone())
                    .key(snap)
                    .presigned(PresigningConfig::expires_in(Duration::from_secs(10 * 60))?)
                    .await?
                    .uri()
                    .clone();
                results
                    .entry(id)
                    .or_insert(Vec::new())
                    .push(object_uri.to_string());
            }
        }
        Ok(results)
    }
}

#[ComplexObject]
impl DataCollection {
    /// Fetched all crystal snapshots and generates s3 URLs
    async fn crystal_snapshots(
        &self,
        ctx: &Context<'_>,
    ) -> async_graphql::Result<Option<Vec<String>>, async_graphql::Error> {
        let loader = ctx.data_unchecked::<DataLoader<CrystalSnapshotDataLoader>>();
        loader.load_one(self.id).await
    }
}

#[Object]
impl Query {
    /// Reference datasets resolver for the router
    #[graphql(entity)]
    async fn router_data_collection(&self, id: u32) -> DataCollection {
        DataCollection { id }
    }
}
