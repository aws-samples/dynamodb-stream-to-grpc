pub mod ddbstream {
    tonic::include_proto!("ddbstream");
}
pub mod error;

use aws_sdk_dynamodb::{model::AttributeValue, model::Select, Client as DynamoDb};
use aws_sdk_kinesis::{model::ShardIteratorType::Latest, Client as Kinesis};
use ddbstream::ddb_stream_server::{DdbStream, DdbStreamServer};
use ddbstream::{SubscribeRequest, SubscribeResponse};
use error::{PollKinesisError, QueryDynamoDbError};
use futures::Stream;
use once_cell::sync::OnceCell;
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::{pin::Pin, sync::Arc};
use tokio::sync::mpsc::{self, Sender};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Server, Request, Response, Status};

type Subscribers = Vec<Sender<Result<SubscribeResponse, Status>>>;

static SUBSCRIBERS: OnceCell<Arc<Mutex<Subscribers>>> = OnceCell::new();

#[derive(Default)]
struct DdbStreamImpl;

#[tonic::async_trait]
impl DdbStream for DdbStreamImpl {
    type SubscribeStream = Pin<Box<dyn Stream<Item = Result<SubscribeResponse, Status>> + Send>>;

    async fn subscribe(
        &self,
        _request: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let (tx, rx) = mpsc::channel(128);

        let subscribers = SUBSCRIBERS.get().unwrap().clone();
        let mut subscribers = subscribers.lock().await;

        subscribers.push(tx.clone());

        Ok(Response::new(
            Box::pin(ReceiverStream::new(rx)) as Self::SubscribeStream
        ))
    }
}

#[derive(Debug, Serialize)]
struct Item {
    id: String,
    value: Option<f64>,
}

async fn query_item(
    client: &DynamoDb,
    id: &str,
) -> Result<HashMap<String, AttributeValue>, QueryDynamoDbError> {
    let res = client
        .query()
        .table_name(env::var("DYNAMODB_TABLE").unwrap())
        .key_condition_expression("#id = :id".to_owned())
        .expression_attribute_names("#id", "id".to_owned())
        .expression_attribute_values(":id", AttributeValue::S(id.to_owned()))
        .select(Select::AllAttributes)
        .send()
        .await?;

    if let Some(mut items) = res.items {
        if let Some(item) = items.pop() {
            return Ok(item);
        }
    }

    Err(QueryDynamoDbError {
        message: "item not found".to_owned(),
    })
}

async fn fetch_shards(client: &Kinesis) -> Result<Vec<String>, PollKinesisError> {
    Ok(client
        .list_shards()
        .stream_name(env::var("KINESIS_STREAM").unwrap())
        .send()
        .await?
        .shards()
        .unwrap()
        .to_vec()
        .iter()
        .map(|s| s.shard_id().unwrap().to_owned())
        .collect())
}

async fn fetch_iterator(client: &Kinesis, shard_id: &str) -> Result<String, PollKinesisError> {
    Ok(client
        .get_shard_iterator()
        .stream_name(env::var("KINESIS_STREAM").unwrap())
        .shard_id(shard_id)
        .shard_iterator_type(Latest)
        .send()
        .await?
        .shard_iterator()
        .unwrap()
        .to_owned())
}

async fn poll_kinesis(client: &Kinesis, dynamodb: &DynamoDb) -> Result<(), PollKinesisError> {
    let subscribers = SUBSCRIBERS.get().unwrap().clone();
    let shards = fetch_shards(&client).await?;
    let mut iterators: Vec<String> =
        futures::future::join_all(shards.iter().map(|s| fetch_iterator(&client, s)))
            .await
            .into_iter()
            .collect::<Result<Vec<String>, PollKinesisError>>()?;

    loop {
        let mut disconnected = Vec::new();
        let mut tmp_iters = Vec::new();

        for iter in iterators {
            let records = client.get_records().shard_iterator(&iter).send().await?;

            if let Some(next_iter) = records.next_shard_iterator() {
                tmp_iters.push(next_iter.to_owned());
            }

            let records = records
                .records()
                .unwrap()
                .iter()
                .map(|r| r.data().unwrap().clone());

            let mut subscribers = subscribers.lock().await;

            for r in records {
                let r_json: serde_json::Value =
                    serde_json::from_str(std::str::from_utf8(r.as_ref()).unwrap()).unwrap();

                let id = r_json["dynamodb"]["Keys"]["id"]["S"].as_str().unwrap();
                let item = query_item(dynamodb, id).await?;
                let item = Item {
                    id: item.get("id").unwrap().as_s().unwrap().to_owned(),
                    value: item
                        .get("value")
                        .and_then(|n| n.as_n().ok())
                        .and_then(|n| n.parse().ok()),
                };

                for (i, tx) in subscribers.iter().enumerate() {
                    if let Err(_) = tx
                        .send(Result::<_, Status>::Ok(SubscribeResponse {
                            r#type: "broadcast".to_owned(),
                            data: serde_json::to_string(&item).unwrap(),
                        }))
                        .await
                    {
                        disconnected.push(i);
                    }
                }
            }

            while let Some(i) = disconnected.pop() {
                subscribers.remove(i);
            }
        }

        iterators = tmp_iters;

        sleep(Duration::from_secs(1)).await;
    }
}

async fn ping_subscribers() {
    let subscribers = SUBSCRIBERS.get().unwrap().clone();

    loop {
        let mut subscribers = subscribers.lock().await;
        let mut disconnected = Vec::new();

        for (i, tx) in subscribers.iter().enumerate() {
            if let Err(_) = tx
                .send(Result::<_, Status>::Ok(SubscribeResponse {
                    r#type: "ping".to_owned(),
                    data: "".to_owned(),
                }))
                .await
            {
                disconnected.push(i);
            }
        }

        while let Some(i) = disconnected.pop() {
            subscribers.remove(i);
        }

        sleep(Duration::from_secs(10)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SUBSCRIBERS.set(Arc::new(Mutex::new(Vec::new()))).unwrap();

    let addr = "0.0.0.0:50051".parse().unwrap();
    let server = DdbStreamImpl::default();

    println!("ddbstream server listening on {}", addr);

    let shared_config = aws_config::load_from_env().await;
    let kinesis = Kinesis::new(&shared_config);
    let dynamodb = DynamoDb::new(&shared_config);

    let t_poll_kinesis = tokio::spawn(async move {
        if let Err(e) = poll_kinesis(&kinesis, &dynamodb).await {
            eprintln!("{}", e);
            panic!();
        }
    });

    let t_ping_subscribers = tokio::spawn(async move {
        ping_subscribers().await;
    });

    Server::builder()
        .add_service(DdbStreamServer::new(server))
        .serve(addr)
        .await?;

    let _ = futures::future::join(t_poll_kinesis, t_ping_subscribers).await;

    Ok(())
}
