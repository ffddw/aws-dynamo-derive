#![doc = include_str!("../README.md")]

mod container;
mod dynamo;
mod item;
mod table;
mod util;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for AwsDynamoDb table
///
/// Table name can be set by adding `#[aws_dynamo(table_name = "AwesomeFooTable")]` attribute on top of the struct.
/// Annotating `#[aws_dynamo(hash_key)]` or `#[aws_dynamo(range_key)]` can set primary of table.
/// As the spec of aws dynamo db, only one hash key is available per table, and 0 or 1 additional range key is available.
/// It wouldn't compile if the key constraint is wrong.
///
/// #### Example
/// ```rust
/// use aws_dynamo_derive::{Table, Item};
/// #[derive(Table)]
/// #[aws_dynamo(table_name = "AwesomeFooTable")]
/// struct FooTable {
///     #[aws_dynamo(range_key)]
///     index: u64,
///     #[aws_dynamo(hash_key)]
///     #[aws_dynamo(global_secondary_index(index_name = "foo_index_1", hash_key))]
///     name: String,
///     temp: i128,
///     values: Values,
/// }
///
/// // values must implements Clone
/// #[derive(Item, Clone)]
/// struct Values {
///     count: u32,
///     count2: u64,
/// }
/// ```
///
/// #### CreateTable Example
///
/// ```rust,ignore
/// async fn create_table() {
///     // accepts CreateTableFluentBuilder
///     let create_table_builder = FooTable::create_table(client.create_table())
///         .local_secondary_indexes(lsi_builder)
///         .global_secondary_indexes(gsi_builder)
///         .provisioned_throughput(provisioned_throughput)
///         .send()
///         .await?;
/// }
///```
/// In order to set LocalSecondaryIndex, annotate the field with `#[aws_dynamo(local_secondary_index(index_name = "foo_index_1", hash_key))]`.
/// LSI can be retrieved using method `get_local_secondary_index_key_schemas` automatically derived by macro.
/// It is imperative that you set the local secondary index along with the CreateTableFluentBuilder if you have LSIs. (https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/LCICli.html#LCICli.CreateTableWithIndex)
/// In order to set GlobalSecondaryIndex, annotate the field with `#[aws_dynamo(global_secondary_index(index_name = "foo_index_1", hash_key))]`.
/// GSI can be retrieved using method `get_global_secondary_index_key_schemas` automatically derived by macro.
///
/// #### LSI Example
/// ```rust,ignore
/// async fn create_lsi() {
///     // returns HashMap
///     let lsi_key_schemas = FooTable::get_local_secondary_index_key_schemas();
///     let lsi_builder = LocalSecondaryIndex::builder()
///         .index_name(idx_name)
///         // defined with attribute
///         .set_key_schema(Some(lsi_key_schemas.get("foo_index_1").unwrap().clone()))
///         .projection(Projection::builder()
///             .projection_type(ProjectionType::All)
///             .build(),
///         )
///         .build()
///         .unwrap();
/// }
/// ```
///
/// #### GSI Example
/// ```rust,ignore
/// async fn create_gsi() {
///     // returns HashMap
///     let gsi_key_schemas = FooTable::get_global_secondary_index_key_schemas();
///     let gsi_builder = GlobalSecondaryIndex::builder()
///         .index_name(idx_name)
///         // defined with attribute
///         .set_key_schema(Some(gsi_key_schemas.get("foo_index_1").unwrap().clone()))
///         .provisioned_throughput(provisioned_throughput.clone())
///         .projection(Projection::builder()
///             .projection_type(ProjectionType::All)
///             .build(),
///         )
///         .build()
///         .unwrap();
/// }
/// ```
#[proc_macro_derive(Table, attributes(aws_dynamo))]
pub fn derive_table(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    table::expand_table(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derive macro for AwsDynamoDb Item
///
/// Derives function to convert rust types to aws dynamo AttributeValue types.
/// Nesting structs is available which converts fields to `AttributeValue::M` type.
///
/// #### Example
/// ```rust,ignore
/// // values must implements Clone
/// #[derive(Item, Clone)]
/// struct Values {
///     count: u32,
///     count2: u64,
/// }
///
/// async fn put_item() {
///     let values = Values {
///         count: 1,
///         count2: 2
///     };
///
///     let foo = FooTable {
///         index: 1,
///         name: "foo".to_string(),
///         // nested struct derives `Item` converted into AttributeValue::M
///         value,
///     };
///
///     let config = aws_config::load_from_env().await;
///     let client = aws_sdk_dynamodb::Client::new(&config);
///     foo.put_item(client.put_item()).send().await?;
/// }
/// ```
/// #### GetItem with PrimaryKey
/// ```rust,ignore
/// async fn get_item_by_primary_key() {
///     // macro expands input struct for primary key `FooTablePrimaryKey`
///     let primary_key = FooTable::get_primary_keys(FooTablePrimaryKey {
///         index: 1,
///         name: "foo".to_string()
///     });
///
///     // query with primary keys
///     // aws_sdk_dynamodb::Client
///     let resp = client
///         .get_item()
///         .table_name(FooTable::get_table_name())
///         .set_key(Some(primary_key))
///         .send()
///         .await?
///         .item();
///
///     let item = resp.item().unwrap();
///     // returns error if type conversion is invalid
///     let converted = FooTable::from_attribute_value(item).unwrap();
/// }
/// ```
/// You can find how the macro handles for other types on README
#[proc_macro_derive(Item, attributes(aws_dynamo))]
pub fn derive_item(input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    item::expand_item(&mut input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
