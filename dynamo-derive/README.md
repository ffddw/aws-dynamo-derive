# dynamo-derive

helper crate for [aws-sdk-dynamodb](https://docs.rs/aws-sdk-dynamodb/latest/aws_sdk_dynamodb/)

Generates conversion codes from rust primitive types to aws dynamo types.

## Examples
```rust
#[tokio::test]
async fn main() {
    #[derive(Table)]
    #[table(table_name = "AwesomeFooTable")]
    struct FooTable<'a> {
        #[table(range_key("N"))]
        index: u64,
        #[table(hash_key("S"))]
        name: &'a str,
    }

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);
    let create_table_builder = FooTable::create_table(client.create_table());
    // do some extra works with create_table_builder
    let _ = create_table_builder.send().await;

    let foo = FooTable {
        index: 1,
        name: "foo",
    };
    let put_item_builder = foo.put_item(client.put_item());
    // do some extra works with put_item_builder
    let _ = put_item_builder.send();
}
```

### KeySchemas and AttributeDefinitions
Struct fields decorated with `#[table(range_key("N"))]` adds `ScalarAttributeType::N` AttributionDefinitions
as well as `KeyType::Range` KeySchema. 

Available KeySchema
- `range_key`
- `hash_key`

Available AttributeDefinition
- `"B"`
- `"N"`
- `"S"`

### AttributeValue
- `&str`, `String` -> `S`
- `bool` -> `BOOL`
- `aws_sdk_dynamodb::primitives::Blob` -> `B`
- `i8 | u8 | .. | u128` -> `N`
- for T: String | &str, `Vec<T>`, `[T; 1]`, `&[T]` -> `SS`
- for T: `i8 | u8 | .. | u128`, `Vec<T>`, `[T; 1]`, `&[T]` -> `NS`
- `Option<()>` -> `NULL`
- if T is `Vec<T>` | `[T; 1]` | `&[T]` but not `SS` nor `NS` -> `L`
- `HashMap<String, T>` -> `M`, automatically converts inner value of HashMap to AttributeValue types.

### Downsides
Macro tries to convert all possible types that leeds to extra allocation while iterating items of collection types like Vector or HashMap.
If the type is super complex and heavy, you might need benchmark before using it. 