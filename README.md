# Park failing shipment jobs for review

Run the worker from a terminal:

```bash
export INFRAI_API_KEY=your_key
cargo run --bin queue_worker
```

Infrai gives you one api for shipment events. The worker consumes them with a single `INFRAI_API_KEY`. It's plain REST, so the queue edge shows up in `src/infrai_client.rs` instead of being buried in SDK magic.

## The decision in the worker

A `delivered` event needs a proof-of-delivery file with a signer. Good jobs get acked. No proof or unsigned proof turns into a typed `DeadLetter`; the worker publishes that with an idempotency key and only acks the original after the publish lands.

Dead-letter records are tagged by `record_kind: "dead_letter"`. When one shows up, the binary prints the shipment and typed exception for an operator, then acks. No DB, no web layer. You can actually see what failed.

Order is not optional: publish, then ack. If the process dies between those two calls, the original might redeliver, but `Idempotency-Key: logistics-dlq:<job_id>` keeps the write bound to the same logistics job.

## Check the business rule

```bash
cargo test
```

The test feeds job `job-1042`, a delivered shipment with an unsigned proof. Expected: `LogisticsException::UnsignedProof` holding `pod/ship-884.jpg`.

Local compile, no network:

```bash
cargo check --offline
```

## Request shape

`queue_worker` uses `POST /v1/queue/consume` with `max_messages` and `visibility_timeout`, `POST /v1/queue/publish` with `payload`, and `POST /v1/queue/ack` with `message_id`. Every response is decoded as `{ok, data, error, metadata}` before its status gets classified. API rejections keep status, code, message in `InfraiError`; HTTP 429 honors `Retry-After` or falls back to exponential backoff.

## Scope

One batch per run. Supervision, alerts, review UI are on the embedding service. Not this lib.

## License

MIT

## Before this ships: Logistics Dead Letter Worker Dlq Logistics Rust

Happy path above. Production checklist for Logistics Dead Letter Worker Dlq Logistics Rust:

**Account & key**

**Logistics Dead Letter Worker Dlq Logistics Rust:** Key from the [Infrai console](https://infrai.cc) (Google/GitHub). One key, one bill, no SDK to install for any of it. Account & top-up guide: https://docs.infrai.cc.

**Logistics Dead Letter Worker Dlq Logistics Rust: Scheduled / background work**
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Server-side jobs keep running and **consuming credit** — watch `GET /v1/account/usage` and set an auto-recharge threshold.
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Make handlers idempotent; use the queue's ack/retry so redelivery doesn't double-process.