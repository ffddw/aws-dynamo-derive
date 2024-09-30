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
```rust,ignore
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

### LocalSecondaryIndex

KeySchemas and AttributeDefinitions for LSIs are parsed and expanded to `create_table()` if you use the following macros:

* Specify a HashKey for an LSI: `#[aws_dynamo(local_secondary_index(index_name = "lsi1", hash_key))]`
  * You should also note that LSI must have the same hash key as the main table.
* Specify a RangeKey for an LSI: `#[aws_dynamo(local_secondary_index(index_name = "lsi1", range_key))]`

If you specify LSIs with above macros, you must attach `LocalSecondaryIndexBuilder`s to `CreateTableFluentBuilder` so that the LSI is created upon table creation.
You can simply get `Vec<KeySchemaElement>` using `get_local_secondary_index_key_schemas()` and pass it to `set_key_schema()` method of `LocalSecondaryIndexBuilder`.

Take a look at [test_local()](aws-dynamo-derive/tests/table.rs) to learn how to use LSIs.

### GlobalSecondaryIndex

KeySchemas and AttributeDefinitions for GSIs are parsed and expanded to `create_table()` if you use the following macros:

* Specify a HashKey for an GSI: `#[aws_dynamo(global_secondary_index(index_name = "gsi1", hash_key))]`
* Specify a RangeKey for an GSI: `#[aws_dynamo(global_secondary_index(index_name = "gsi1", range_key))]`

If you specify GSIs with above macros, you must attach `GlobalSecondaryIndexBuilder`s to `CreateTableFluentBuilder` so that the GSI is created upon table creation.
You can simply get `Vec<KeySchemaElement>` using `get_global_secondary_index_key_schemas()` and pass it to `set_key_schema()` method of `GlobalSecondaryIndexBuilder`.

Take a look at [test_local()](aws-dynamo-derive/tests/table.rs) to learn how to use GSIs.

### AttributeValue conversions

`from_attribute_value` converts `HashMap<String, AttributeValue>` to Rust types. 
If any field type does not match the given `AttributeValue` type, it returns `Err(AttributeValue)`.

### Downsides

The macro tries to convert all possible types, which leads to extra allocation while iterating items of collection types like `Vector` or `HashMap`. 
If the type is super complex and heavy, you might need to benchmark before using it.
