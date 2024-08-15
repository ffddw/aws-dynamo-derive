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
        temp: i128,
        values: Values,
    }
    
    // values must implements Clone
    #[derive(Item, Clone)]
    struct Values {
        count: u32,
        count2: u64,
    }
}
```

### Create Table
```rust
async fn create_table() {
    // accepts CreateTableFluentBuilder
    let create_table_builder = FooTable::create_table(client.create_table())
        .global_secondary_indexes(gsi_builder)
        .provisioned_throughput(provisioned_throughput)
        .send()
        .await?;
}

```

### GlobalSecondaryIndexes
```rust
async fn create_gsi() {
    // returns HashMap
    let gsi_key_schemas = FooTable::get_global_secondary_index_key_schemas();
    let gsi_builder = GlobalSecondaryIndex::builder()
        .index_name(idx_name)
        // defined with attribute
        .set_key_schema(Some(gsi_key_schemas.get("foo_index_1").unwrap().clone()))
        .provisioned_throughput(provisioned_throughput.clone())
        .projection(Projection::builder()
            .projection_type(ProjectionType::All)
            .build(),
        )
        .build()
        .unwrap();
}
```

### PutItem
```rust
async fn put_item() {
    let values = Values {
        count: 1,
        count2: 2
    };

    let foo = FooTable {
        index: 1,
        name: "foo".to_string(),
        // nested struct turns into AttributeValue::M
        value,
    };

    foo.put_item(client.put_item()).send().await?;
}
```

### GetItem with PrimaryKey
```rust
async fn get_item_by_primary_key() {
    // macro expands input struct for primary key `FooTablePrimaryKey`
    let primary_key = FooTable::get_primary_keys(FooTablePrimaryKey {
        index: 1,
        name: "foo".to_string()
    });

    // query with primary keys
    let resp = client
        .get_item()
        .table_name(FooTable::get_table_name())
        .set_key(Some(primary_key))
        .send()
        .await?
        .item();
    
    let item = resp.item().unwrap();
    // returns error if type conversion is invalid
    let converted = FooTable::from_attribute_value(item).unwrap();
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
