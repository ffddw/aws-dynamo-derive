use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::Client;
use crab_box_dynamo_derive::{Item, Table};
use std::collections::HashMap;

#[tokio::test]
async fn test_conversions() {
    #[derive(Table, Debug, Eq, PartialEq)]
    struct Outer {
        #[table(hash_key)]
        hk: String,
        inner: Vec<Inner>,
    }

    #[derive(Item, Clone, Debug, Eq, PartialEq)]
    struct Inner {
        name: String,
        value: u32,
    }

    let expected_outer = Outer {
        hk: "abc".to_string(),
        inner: vec![Inner {
            name: "foo".to_string(),
            value: 1,
        }],
    };

    let config = aws_config::load_from_env().await;
    let client = Client::new(&config);

    let builder = expected_outer.put_item(client.put_item());
    let item = builder.get_item().as_ref().unwrap();

    let mut expected_map = HashMap::new();
    expected_map.insert("Hk".to_string(), AttributeValue::S("abc".to_string()));
    let mut inner_map = HashMap::new();
    inner_map.insert("Name".to_string(), AttributeValue::S("foo".to_string()));
    inner_map.insert("Value".to_string(), AttributeValue::N("1".to_string()));
    expected_map.insert(
        "Inner".to_string(),
        AttributeValue::L(vec![AttributeValue::M(inner_map)]),
    );

    assert_eq!(item.get("HK"), expected_map.get("HK"));
    assert_eq!(item.get("Inner"), expected_map.get("Inner"));

    let outer = Outer::from_attribute_value(&expected_map).unwrap();
    assert_eq!(outer, expected_outer);
}
