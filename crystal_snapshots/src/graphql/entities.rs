use async_graphql::SimpleObject;

/// Combines autoproc integration, autoproc program, autoproc and autoproc scaling
#[derive(Debug, Clone, SimpleObject)]
#[graphql(name = "DataCollection", unresolvable = "dataCollectionId", complex)]
pub struct DataCollection {
    // #[graphql(skip)]
    pub id: u32,
}
