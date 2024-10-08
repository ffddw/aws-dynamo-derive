use std::collections::HashMap;

use aws_sdk_dynamodb::primitives::Blob;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, GlobalSecondaryIndex, KeySchemaElement, KeyType,
    LocalSecondaryIndex, Projection, ProjectionType, ProvisionedThroughput, ScalarAttributeType,
};
use aws_sdk_dynamodb::Client;

use aws_dynamo_derive::Table;

/// ## Compile fail cases
/// ```compile_fail
/// #[derive(Table)]
///     struct Table {
///         #[aws_dynamo(hash_key)]
///         hash_key: String,
///         #[aws_dynamo(hash_key)] // compile fails: only one HashKey is allowed
///         duplicated_hash_key: String,
///     }
///
///
/// #[derive(Table)]
///     struct Table {
///         #[aws_dynamo(hash_key)]
///         hash_key: String,
///         #[aws_dynamo(range_key)]
///         range_key: u32,
///         #[aws_dynamo(range_key)]
///         duplicated_range_key: u32, // compile fails: at most one RangeKey is allowed
///     }

#[tokio::test]
async fn test_create_table_and_put_item() {
    #[derive(Table)]
    #[aws_dynamo(table_name = "AwesomeFooTable")]
    pub struct FooTable {
        #[aws_dynamo(range_key)]
        #[aws_dynamo(global_secondary_index(index_name = "gsi1", range_key))]
        range_key: u32,
        #[aws_dynamo(hash_key)]
        #[aws_dynamo(local_secondary_index(index_name = "lsi1", hash_key))]
        primary: String,
        #[aws_dynamo(global_secondary_index(index_name = "gsi1", hash_key))]
        hash_key: String,
        #[aws_dynamo(global_secondary_index(index_name = "gsi2", hash_key))]
        #[aws_dynamo(local_secondary_index(index_name = "lsi1", range_key))]
        gsi_idx: String,
        a: Vec<Vec<Vec<String>>>,
        bool: bool,
        blob: Vec<Vec<Blob>>,
        null: Option<()>,
        nulls: Vec<Option<()>>,
        map: HashMap<String, Vec<HashMap<String, String>>>,
    }

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);
    let builder = FooTable::create_table(client.create_table());
    assert_eq!(
        builder.get_table_name().as_ref().unwrap(),
        "AwesomeFooTable"
    );
    let key_schemas = builder.get_key_schema().as_ref().unwrap();
    // tests HashKey always prior to RangeKey
    assert_eq!(
        key_schemas,
        &vec![
            KeySchemaElement::builder()
                .attribute_name("Primary")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
            KeySchemaElement::builder()
                .attribute_name("RangeKey")
                .key_type(KeyType::Range)
                .build()
                .unwrap()
        ]
    );
    let attribute_definitions = builder.get_attribute_definitions().as_ref().unwrap();
    assert_eq!(
        attribute_definitions,
        &vec![
            AttributeDefinition::builder()
                .attribute_name("RangeKey")
                .attribute_type(ScalarAttributeType::N)
                .build()
                .unwrap(),
            AttributeDefinition::builder()
                .attribute_name("Primary")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
            AttributeDefinition::builder()
                .attribute_name("HashKey")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
            AttributeDefinition::builder()
                .attribute_name("GsiIdx")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
        ]
    );

    let mut map = HashMap::new();
    let mut inner_map = HashMap::new();
    inner_map.insert("2".to_string(), "b".to_string());
    map.insert("1".to_string(), vec![inner_map]);

    let foo_table = FooTable {
        primary: "primary".to_string(),
        hash_key: "hash_key".to_string(),
        range_key: 1,
        blob: vec![vec![Blob::new(vec![])]],
        null: None,
        nulls: vec![Some(()), None],
        map,
        gsi_idx: "gsi_idx".to_string(),
        a: vec![vec![vec!["1".to_string()]]],
        bool: false,
    };

    let builder = foo_table.put_item(client.put_item());
    assert_eq!(
        builder.get_table_name().as_ref().unwrap(),
        "AwesomeFooTable"
    );
    let item = builder.get_item().as_ref().unwrap();
    assert_eq!(
        item.get("A").unwrap(),
        &AttributeValue::L(vec![AttributeValue::L(vec![AttributeValue::Ss(vec![
            "1".to_string()
        ])])]),
    );
    assert_eq!(
        item.get("Blob").unwrap(),
        &AttributeValue::L(vec![AttributeValue::Bs(vec![Blob::new(vec![])])]),
    );
    assert_eq!(item.get("Bool").unwrap(), &AttributeValue::Bool(false));
    assert_eq!(item.get("Null").unwrap(), &AttributeValue::Null(true));

    let mut expected_map = HashMap::new();
    let mut inner_expected_map = HashMap::new();
    inner_expected_map.insert("2".to_string(), AttributeValue::S("b".to_string()));
    expected_map.insert(
        "1".to_string(),
        AttributeValue::L(vec![AttributeValue::M(inner_expected_map)]),
    );
    assert_eq!(item.get("Map").unwrap(), &AttributeValue::M(expected_map));

    let local_secondary_indexes = FooTable::get_local_secondary_index_key_schemas();
    let idx_lsi = local_secondary_indexes.get("lsi1").unwrap();
    assert_eq!(
        idx_lsi,
        &vec![
            KeySchemaElement::builder()
                .attribute_name("Primary")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
            KeySchemaElement::builder()
                .attribute_name("GsiIdx")
                .key_type(KeyType::Range)
                .build()
                .unwrap()
        ]
    );

    let global_secondary_indexes = FooTable::get_global_secondary_index_key_schemas();
    let idx_gsi = global_secondary_indexes.get("gsi1").unwrap();
    assert_eq!(
        idx_gsi,
        &vec![
            KeySchemaElement::builder()
                .attribute_name("HashKey")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
            KeySchemaElement::builder()
                .attribute_name("RangeKey")
                .key_type(KeyType::Range)
                .build()
                .unwrap()
        ]
    );
}

#[tokio::test]
async fn attribute_value_to_rust_types() {
    #[derive(Debug, Table, Eq, PartialEq)]
    pub struct FooTable {
        #[aws_dynamo(hash_key)]
        hash_key: String,
        num: u32,
        vec_of_num: Vec<u128>,
        vec_of_string: Vec<Vec<String>>,
        nested_vec_of_num: Vec<Vec<u16>>,
        map: HashMap<String, Vec<i8>>,
        nested_vec_of_map: Vec<HashMap<String, u128>>,
        nulls: Vec<Option<()>>,
        blobs: Vec<Blob>,
        blob: Blob,
        bool: bool,
        bools: Vec<bool>,
    }

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let mut map = HashMap::new();
    map.insert("key".to_string(), vec![7]);

    let mut map2 = HashMap::new();
    map2.insert("key2".to_string(), 9);

    let foo_table = FooTable {
        hash_key: "hash_key".to_string(),
        num: 1,
        vec_of_num: vec![1, 2],
        vec_of_string: vec![vec!["3".to_string(), "4".to_string()]],
        nested_vec_of_num: vec![vec![5, 6]],
        map,
        nested_vec_of_map: vec![map2],
        nulls: vec![Some(()), None],
        blob: Blob::new(vec![]),
        blobs: vec![Blob::new(vec![])],
        bool: true,
        bools: vec![],
    };

    let builder = client.put_item();
    let items = FooTable::put_item(&foo_table, builder)
        .get_item()
        .as_ref()
        .unwrap()
        .clone();

    let foo_table2 = FooTable::from_attribute_value(&items).unwrap();
    let foo_table3: FooTable = items.try_into().unwrap();

    assert_eq!(foo_table, foo_table2);
    assert_eq!(foo_table2, foo_table3);
}

#[tokio::test]
async fn attribute_value_to_rust_types_checks() {
    #[derive(Debug, Table, Eq, PartialEq)]
    pub struct FooTable {
        #[aws_dynamo(hash_key)]
        hash_key: String,
        vec_of_num: Vec<u128>,
    }

    let foo_table = FooTable {
        hash_key: "hk".to_string(),
        vec_of_num: vec![1],
    };

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let builder = client.put_item();
    let mut items = FooTable::put_item(&foo_table, builder)
        .get_item()
        .as_ref()
        .unwrap()
        .clone();

    items
        .entry("HashKey".to_string())
        .and_modify(|hk| *hk = AttributeValue::N("wrong".to_string()));

    // if the table attribute is not matched with the given value, returns error
    let res = FooTable::from_attribute_value(&items);
    assert!(res.is_err());
}

#[tokio::test]
async fn test_get_primary_keys() {
    #[derive(Debug, Table, Eq, PartialEq)]
    pub struct FooTable {
        #[aws_dynamo(range_key)]
        range_key: u32,
        #[aws_dynamo(hash_key)]
        hash_key: String,
    }

    let _foo_table = FooTable {
        hash_key: "hk".to_string(),
        range_key: 1,
    };

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);
    let primary_key = FooTable::get_primary_keys(FooTablePrimaryKey {
        range_key: 1,
        hash_key: "hk".to_string(),
    });

    let mut expected_map = HashMap::new();
    expected_map.insert("RangeKey".to_string(), AttributeValue::N(1.to_string()));
    expected_map.insert("HashKey".to_string(), AttributeValue::S("hk".to_string()));

    assert_eq!(primary_key, expected_map);

    // compiles well
    let _ = client
        .get_item()
        .table_name(FooTable::get_table_name())
        .set_key(Some(primary_key));
}

// docker run -p 8000:8000 --rm amazon/dynamodb-local
#[ignore]
#[tokio::test]
async fn test_local() {
    std::env::set_var("LOCAL_DYNAMO_URL", "http://localhost:8000");

    #[derive(Table)]
    #[aws_dynamo(table_name = "AwesomeFooTable")]
    pub struct FooTable {
        #[aws_dynamo(range_key)]
        #[aws_dynamo(global_secondary_index(index_name = "gsi1", range_key))]
        range_key: u32,
        #[aws_dynamo(hash_key)]
        #[aws_dynamo(local_secondary_index(index_name = "lsi1", hash_key))]
        primary: String,
        #[aws_dynamo(global_secondary_index(index_name = "gsi1", hash_key))]
        hash_key: String,
        #[aws_dynamo(global_secondary_index(index_name = "gsi2", hash_key))]
        #[aws_dynamo(local_secondary_index(index_name = "lsi1", range_key))]
        gsi_idx: String,
        a: Vec<Vec<Vec<String>>>,
        bool: bool,
        blob: Vec<Vec<Blob>>,
        null: Option<()>,
        nulls: Vec<Option<()>>,
        map: HashMap<String, Vec<HashMap<String, String>>>,
    }

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let lsi_key_schemas = FooTable::get_local_secondary_index_key_schemas();
    let lsi_builder = LocalSecondaryIndex::builder()
        .index_name("lsi1")
        .set_key_schema(Some(lsi_key_schemas.get("lsi1").unwrap().clone()))
        .projection(
            Projection::builder()
                .projection_type(ProjectionType::All)
                .build(),
        )
        .build()
        .unwrap();

    let gsi_key_schemas = FooTable::get_global_secondary_index_key_schemas();
    let gsi_builder_1 = GlobalSecondaryIndex::builder()
        .index_name("gsi1")
        .set_key_schema(Some(gsi_key_schemas.get("gsi1").unwrap().clone()))
        .provisioned_throughput(
            ProvisionedThroughput::builder()
                .read_capacity_units(1)
                .write_capacity_units(1)
                .build()
                .unwrap(),
        )
        .projection(
            Projection::builder()
                .projection_type(ProjectionType::All)
                .build(),
        )
        .build()
        .unwrap();
    let gsi_builder_2 = GlobalSecondaryIndex::builder()
        .index_name("gsi2")
        .set_key_schema(Some(gsi_key_schemas.get("gsi2").unwrap().clone()))
        .provisioned_throughput(
            ProvisionedThroughput::builder()
                .read_capacity_units(1)
                .write_capacity_units(1)
                .build()
                .unwrap(),
        )
        .projection(
            Projection::builder()
                .projection_type(ProjectionType::All)
                .build(),
        )
        .build()
        .unwrap();

    FooTable::create_table(client.create_table())
        .local_secondary_indexes(lsi_builder)
        .global_secondary_indexes(gsi_builder_1)
        .global_secondary_indexes(gsi_builder_2)
        .provisioned_throughput(
            ProvisionedThroughput::builder()
                .read_capacity_units(1)
                .write_capacity_units(1)
                .build()
                .unwrap(),
        )
        .send()
        .await
        .unwrap();
}
