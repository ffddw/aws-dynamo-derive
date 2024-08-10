use aws_sdk_dynamodb::primitives::Blob;
use aws_sdk_dynamodb::types::{
    AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ScalarAttributeType,
};
use aws_sdk_dynamodb::Client;
use crab_box_dynamo_derive::Table;
use std::collections::HashMap;

#[tokio::test]
async fn test_create_table() {
    #[derive(Table)]
    #[table(table_name = "AwesomeFooTable")]
    struct FooTable<'a> {
        #[table(range_key("B"))]
        a: &'a [Vec<[String; 1]>],
        #[table(hash_key("S"))]
        b: &'a [[Vec<String>; 1]],
        c: Vec<&'a [[u8; 1]]>,
        d: Vec<[&'a [i16]; 1]>,
        #[table(hash_key("N"))]
        e: [&'a [Vec<u32>]; 1],
        f: [Vec<&'a [i64]>; 1],
        blob: &'a [Vec<Blob>; 1],
        bool: bool,
        null: Option<()>,
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
    assert_eq!(
        key_schemas,
        &vec![
            KeySchemaElement::builder()
                .attribute_name("B")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
            KeySchemaElement::builder()
                .attribute_name("E")
                .key_type(KeyType::Hash)
                .build()
                .unwrap(),
            KeySchemaElement::builder()
                .attribute_name("A")
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
                .attribute_name("B")
                .attribute_type(ScalarAttributeType::S)
                .build()
                .unwrap(),
            AttributeDefinition::builder()
                .attribute_name("E")
                .attribute_type(ScalarAttributeType::N)
                .build()
                .unwrap(),
            AttributeDefinition::builder()
                .attribute_name("A")
                .attribute_type(ScalarAttributeType::B)
                .build()
                .unwrap(),
        ]
    );

    let mut map = HashMap::new();
    let mut inner_map = HashMap::new();
    inner_map.insert("2".to_string(), "b".to_string());
    map.insert("1".to_string(), vec![inner_map]);

    let foo_table = FooTable {
        a: &[vec![[String::from("1")]]],
        b: &[[vec![String::from("1")]]],
        c: vec![&[[1]]],
        d: vec![[&[1]]],
        e: [&[vec![1]]],
        f: [vec![&[1]]],
        blob: &[vec![Blob::new(vec![])]],
        bool: false,
        null: None,
        map,
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
        item.get("B").unwrap(),
        &AttributeValue::L(vec![AttributeValue::L(vec![AttributeValue::Ss(vec![
            "1".to_string()
        ])])]),
    );
    assert_eq!(
        item.get("C").unwrap(),
        &AttributeValue::L(vec![AttributeValue::L(vec![AttributeValue::Ns(vec![
            "1".to_string()
        ])])]),
    );
    assert_eq!(
        item.get("D").unwrap(),
        &AttributeValue::L(vec![AttributeValue::L(vec![AttributeValue::Ns(vec![
            "1".to_string()
        ])])]),
    );
    assert_eq!(
        item.get("E").unwrap(),
        &AttributeValue::L(vec![AttributeValue::L(vec![AttributeValue::Ns(vec![
            "1".to_string()
        ])])]),
    );
    assert_eq!(
        item.get("F").unwrap(),
        &AttributeValue::L(vec![AttributeValue::L(vec![AttributeValue::Ns(vec![
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
    assert_eq!(item.get("Map").unwrap(), &AttributeValue::M(expected_map))
}
