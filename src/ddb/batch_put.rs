use super::Store;
use anyhow::Result;
use aws_sdk_dynamodb::{
    types::{AttributeValue, PutRequest, ReturnConsumedCapacity, WriteRequest},
    Client,
};
use log::{error, info, trace};
use std::collections::HashMap;

pub async fn batch_store_put(
    client: &Client,
    store: &HashMap<String, Store>,
    table: &str,
) -> Result<()> {
    let ops = store
        .iter()
        .map(|(k, v)| {
            WriteRequest::builder()
                .set_put_request(Some({
                    let mut putreq = PutRequest::builder()
                        .item("store", AttributeValue::S(k.to_string()))
                        .item("attribute", AttributeValue::L(v.attribute.iter().map(|x| AttributeValue::S(x.to_string())).collect::<Vec<AttributeValue>>()));
                    if let Some(version) = &v.version {
                        putreq = putreq.item("version", AttributeValue::S(version.clone()));
                    }
                    putreq.build().expect("Failed to build PutRequest")
                }))
                .build()
        })
        .collect::<Vec<WriteRequest>>();

    // Iterate over 25 items at a time
    let batches = ops.chunks(25);
    info!("Batches: {:?}", batches.len());
    for batch in batches {
        let unprocessed = Some(HashMap::from([(table.to_string(), batch.to_vec())]));
        let out = client
            .batch_write_item()
            .set_request_items(unprocessed)
            .return_consumed_capacity(ReturnConsumedCapacity::Total)
            .send()
            .await;

        if out.is_err() {
            error!("{:?}", out);
            error!("{:?}", batch);
            anyhow::bail!("Failed to batch write items")
        }

        trace!("Results: {:?}", out);
    }

    Ok(())
}
