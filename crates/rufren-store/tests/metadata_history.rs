use rufren_store::Store;
use serde_json::json;

#[test]
fn stored_ink_payload_retains_float_bits_and_serialized_hash_input() {
    // These ordinary coordinate-sized values expose lossy default JSON parsing.
    let payload = json!({"points":[
        {"x":51.248178375505404_f64,"y":52.314008204106244_f64,"pressure":0.0},
        {"x":97.45365320034685_f64,"y":-36.573994842753436_f64,"pressure":0.7}
    ]});
    let before = serde_json::to_vec(&payload).unwrap();
    let store = Store::open(":memory:").unwrap();
    store
        .set("handwriting.artifact:float-regression", &payload)
        .unwrap();
    let restored: serde_json::Value = store
        .get("handwriting.artifact:float-regression")
        .unwrap()
        .unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), before);
}
fn keys(store: &Store, prefix: &str) -> Vec<String> {
    store
        .metadata_with_prefix(prefix)
        .unwrap()
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

#[test]
fn literal_prefix_and_scope_queries_preserve_unrelated_metadata() {
    let store = Store::open(":memory:").unwrap();
    store
        .set(
            "handwriting.activity:a",
            &json!({"scope_id":"one","id":"a"}),
        )
        .unwrap();
    store
        .set(
            "handwriting.activity:b",
            &json!({"scope_id":"two","id":"b"}),
        )
        .unwrap();
    store
        .set(
            "handwriting.artifact:large",
            &json!({"scope_id":"two","payload":"x".repeat(1_000_000)}),
        )
        .unwrap();
    store
        .set("handwriting.activity%:literal", &json!({"scope_id":"one"}))
        .unwrap();
    store
        .set("handwriting.activityX:neighbor", &json!(false))
        .unwrap();
    let activities = store.metadata_with_prefix("handwriting.activity:").unwrap();
    assert_eq!(activities.len(), 2);
    assert_eq!(activities[0].0, "handwriting.activity:a");
    assert_eq!(
        store
            .metadata_with_scope("handwriting.activity:", "one")
            .unwrap(),
        vec![activities[0].clone()]
    );
    assert!(
        store
            .metadata_with_scope("handwriting.artifact:", "one")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        keys(&store, "handwriting.activity%:"),
        vec!["handwriting.activity%:literal"]
    );
    assert_eq!(
        store.get::<bool>("handwriting.activityX:neighbor").unwrap(),
        Some(false)
    );
}

#[test]
fn unicode_prefix_bounds_are_literal_and_handle_last_scalar() {
    let store = Store::open(":memory:").unwrap();
    for key in [
        "А:one",
        "А:two",
        "Б:other",
        "\u{d7ff}:edge",
        "\u{e000}:other",
        "\u{10ffff}last",
    ] {
        store.set(key, &json!(key)).unwrap();
    }
    assert_eq!(keys(&store, "А:"), vec!["А:one", "А:two"]);
    assert_eq!(keys(&store, "\u{d7ff}"), vec!["\u{d7ff}:edge"]);
    assert_eq!(keys(&store, "\u{10ffff}"), vec!["\u{10ffff}last"]);
    assert_eq!(keys(&store, "").len(), 6);
}
