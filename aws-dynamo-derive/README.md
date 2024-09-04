# dynamo-derive

Helper crate for [aws-sdk-dynamodb](https://docs.rs/aws-sdk-dynamodb/latest/aws_sdk_dynamodb/).

Generates conversion codes from Rust primitive types to AWS DynamoDB types.
Works well with nested types!

### Example
```rust
use aws_dynamo_derive::{Item, Table};

#[derive(Table)]
struct Foo {
    #[aws_dynamo(hash_key)]
    pub name: String,
    pub value: Value
}

#[derive(Item, Clone)]
struct Value {
    pub numbers: Vec<u64>,
    pub list_of_ss: Vec<Vec<String>>, 
}
```
this generates
```
{
    "Value": M(
        {
            "Numbers": Ns(["1", "2", "3"]), 
            "ListOfSs": L([Ss(["one"]), Ss(["two"]), Ss(["three"])])
        }
    ), 
    "Name": S("foo_value")
}
```

### KeySchemas and AttributeDefinitions

Struct fields decorated with `#[aws_dynamo(hash_key)]` add `KeyType::Hash` KeySchemas, and by data type of the fields, macro maps 
those to AttributeDefinitions.

Available KeySchemas:

- `range_key`
- `hash_key`

AttributeDefinition mappings:
- `String` -> `S`
- `i8 | u8 | .. | u128` -> `N`
- `Blob` -> `B`

### AttributeValue

- `String` -> `S`
- `bool` -> `BOOL`
- `Blob` -> `B`
- `i8` | `u8` | `..` | `u128` -> `N`
- `Vec<String>` -> `SS`
- For `T`: `i8` | `u8` | `..` | `u128`, `Vec<T>` -> `NS`
- `Vec<Blob>` -> `Bs`
- `Option<()>` -> `NULL`
- If `T` is `Vec<T>` but not `SS` | `NS` | `Bs` -> `L`
- `HashMap<String, T>` -> `M`, automatically converts inner values of `HashMap` to `AttributeValue` types.
- struct that derives `Item` and be converted into `AttributeValue`.

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
