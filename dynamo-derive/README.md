# dynamo-derive

Helper crate for [aws-sdk-dynamodb](https://docs.rs/aws-sdk-dynamodb/latest/aws_sdk_dynamodb/).

Generates conversion codes from Rust primitive types to AWS DynamoDB types.

## Examples

```rust
use crab_box_dynamo_derive::Table;

#[tokio::test]
async fn main() {
    #[derive(Table)]
    #[table(table_name = "AwesomeFooTable")]
    struct FooTable {
        #[table(range_key("N"))]
        index: u64,
        #[table(hash_key("S"))]
        #[table(global_secondary_index(index_name = "foo_index_1", hash_key("S")))]
        name: String,
    }

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let provisioned_throughput = ProvisionedThroughput::builder()
        .read_capacity_units(10)
        .write_capacity_units(30)
        .build()
        .unwrap();

    let idx_name = "foo_index_1";
    let gsi_key_schemas = FooTable::get_global_secondary_index_key_schemas();
    let gsi_builder = GlobalSecondaryIndex::builder()
        .index_name(idx_name)
        .set_key_schema(Some(gsi_key_schemas.get(idx_name).unwrap().clone()))
        .provisioned_throughput(provisioned_throughput.clone())
        .projection(
            Projection::builder()
                .projection_type(ProjectionType::All)
                .build(),
        )
        .build()
        .unwrap();

    // do some extra work with create_table_builder
    let create_table_builder = FooTable::create_table(client.create_table())
        .global_secondary_indexes(gsi_builder)
        .provisioned_throughput(provisioned_throughput);

    let res = create_table_builder.send().await;

    let foo = FooTable {
        index: 1,
        name: "foo".to_string(),
    };

    // do some extra work with put_item_builder
    let put_item_builder = foo.put_item(client.put_item());
    let _ = put_item_builder.send().await;

    // macro expands input struct for primary key
    let primary_key = FooTable::get_primary_keys(FooTablePrimaryKey {
        index: 1,
        name: "foo".to_string()
    });
    
    // query with primary keys
    let _ = client
        .get_item()
        .table_name(FooTable::get_table_name())
        .set_key(Some(primary_key))
        .send()
        .await;
}
```

### KeySchemas and AttributeDefinitions

Struct fields decorated with `#[table(range_key("N"))]` add `ScalarAttributeType::N` AttributeDefinitions as well as `KeyType::Range` KeySchema.

Available KeySchemas:

- `range_key`
- `hash_key`

Available AttributeDefinitions:

- `"B"`
- `"N"`
- `"S"`

### AttributeValue

- `&str`, `String` -> `S`
- `bool` -> `BOOL`
- `aws_sdk_dynamodb::primitives::Blob` -> `B`
- `i8 | u8 | .. | u128` -> `N`
- For `T: String | &str`, `Vec<T>`, `[T; 1]`, `&[T]` -> `SS`
- For `T: i8 | u8 | .. | u128`, `Vec<T>`, `[T; 1]`, `&[T]` -> `NS`
- `Option<()>` -> `NULL`
- If `T` is `Vec<T>` | `[T; 1]` | `&[T]` but not `SS` nor `NS` -> `L`
- `HashMap<String, T>` -> `M`, automatically converts inner values of `HashMap` to `AttributeValue` types.

### GlobalSecondaryIndex

There are many things to set for GSI compared to other fields. This API only provides KeySchemaElement. 
`get_global_secondary_index_key_schemas` returns a `HashMap` of `{ index name: [KeySchemaElement] }` with the value given to the attribute. 
By getting the `Vec<KeySchemaElement>` using the `index_name` as the key of `HashMap`, you can pass the retrieved value to the `set_key_schema` method of `GlobalSecondaryIndexBuilder`.

### AttributeValue conversions

`from_attribute_value` converts `HashMap<String, AttributeValue>` to Rust types. 
If any field type does not match the given `AttributeValue` type, it returns `Err(AttributeValue)`.

### Downsides

The macro tries to convert all possible types, which leads to extra allocation while iterating items of collection types like `Vector` or `HashMap`. 
If the type is super complex and heavy, you might need to benchmark before using it.
