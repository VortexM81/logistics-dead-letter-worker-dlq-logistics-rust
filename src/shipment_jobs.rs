use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "record_kind", rename_all = "snake_case")]
pub enum QueueRecord {
    Shipment(ShipmentEvent),
    DeadLetter(DeadLetter),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShipmentEvent {
    pub job_id: String,
    pub shipment_id: String,
    pub event: ShipmentEventKind,
    pub proof_of_delivery: Vec<ProofOfDeliveryFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShipmentEventKind {
    PickedUp,
    InTransit,
    Delivered,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProofOfDeliveryFile {
    pub object_key: String,
    pub sha256: String,
    pub signed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeadLetter {
    pub original_job_id: String,
    pub shipment_id: String,
    pub exception: LogisticsException,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Error)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LogisticsException {
    #[error("delivered shipment has no proof-of-delivery file")]
    MissingProofOfDelivery,
    #[error("proof-of-delivery file {object_key} has no signer")]
    UnsignedProof { object_key: String },
}

pub fn dead_letter_for(event: ShipmentEvent) -> Result<(), DeadLetter> {
    if event.event != ShipmentEventKind::Delivered {
        return Ok(());
    }

    let exception = match event.proof_of_delivery.first() {
        None => LogisticsException::MissingProofOfDelivery,
        Some(file) if file.signed_by.as_deref().unwrap_or("").trim().is_empty() => {
            LogisticsException::UnsignedProof {
                object_key: file.object_key.clone(),
            }
        }
        Some(_) => return Ok(()),
    };

    Err(DeadLetter {
        original_job_id: event.job_id,
        shipment_id: event.shipment_id,
        exception,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivered_shipment_with_unsigned_proof_becomes_dead_letter() {
        let event = ShipmentEvent {
            job_id: "job-1042".into(),
            shipment_id: "ship-884".into(),
            event: ShipmentEventKind::Delivered,
            proof_of_delivery: vec![ProofOfDeliveryFile {
                object_key: "pod/ship-884.jpg".into(),
                sha256: "8d7f".into(),
                signed_by: None,
            }],
        };

        let dead_letter = dead_letter_for(event).expect_err("unsigned proof must be parked");
        assert_eq!(dead_letter.original_job_id, "job-1042");
        assert_eq!(
            dead_letter.exception,
            LogisticsException::UnsignedProof {
                object_key: "pod/ship-884.jpg".into()
            }
        );
    }
}
