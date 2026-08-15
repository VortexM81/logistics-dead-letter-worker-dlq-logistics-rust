use logistics_dead_letter_worker::{
    infrai_client::{InfraiClient, InfraiError},
    shipment_jobs::{dead_letter_for, QueueRecord},
};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Deserialize)]
struct QueueBatch {
    items: Vec<QueuedMessage>,
}

#[derive(Debug, Deserialize)]
struct QueuedMessage {
    message_id: String,
    payload: QueueRecord,
}

#[derive(Debug, Error)]
enum WorkerError {
    #[error(transparent)]
    Infrai(#[from] InfraiError),
}

#[tokio::main]
async fn main() -> Result<(), WorkerError> {
    let infrai = InfraiClient::from_env()?;
    let batch: QueueBatch = infrai.consume(10, 45).await?;

    for message in batch.items {
        match message.payload {
            QueueRecord::Shipment(event) => match dead_letter_for(event) {
                Ok(()) => {
                    let _: Value = infrai.ack(&message.message_id).await?;
                    println!("completed logistics message {}", message.message_id);
                }
                Err(dead_letter) => {
                    let key = format!("logistics-dlq:{}", dead_letter.original_job_id);
                    let record = QueueRecord::DeadLetter(dead_letter);
                    let _: Value = infrai.publish(&record, &key).await?;
                    let _: Value = infrai.ack(&message.message_id).await?;
                    println!("parked logistics message {}", message.message_id);
                }
            },
            QueueRecord::DeadLetter(dead_letter) => {
                println!(
                    "dead letter ready for review: shipment={}, exception={}",
                    dead_letter.shipment_id, dead_letter.exception
                );
                let _: Value = infrai.ack(&message.message_id).await?;
            }
        }
    }

    Ok(())
}
