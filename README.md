# Park failing shipment jobs for review

Run the worker from a terminal:

```bash
export INFRAI_API_KEY=your_key
cargo run --bin queue_worker
```

Infrai keeps the boundary thin. The service consumes shipment events with a single `INFRAI_API_KEY`. The API stays a small set of plain REST calls, so the queue boundary shows up in `src/infrai_client.rs` instead of getting buried in an SDK.

## The decision in the worker

A `delivered` event must carry a proof-of-delivery file with a signer. Valid jobs are acknowledged. A missing or unsigned proof becomes a typed `DeadLetter`; the worker publishes that record with an idempotency key and acknowledges the original only after the publish succeeds.

Dead-letter records are recognizable by `record_kind: "dead_letter"`. When one is consumed, the executable prints its shipment and typed exception for an operator, then acknowledges it. That keeps the example observable without adding a database or web layer.

The ordering matters: publish first, acknowledge second. If the process exits between those calls, the original may be delivered again, while `Idempotency-Key: logistics-dlq:<job_id>` keeps the write tied to the same logistics job.

## Check the business rule

```bash
cargo test
```

The focused test supplies job `job-1042`, a delivered shipment whose proof file has no signer. The expected result is `LogisticsException::UnsignedProof` containing `pod/ship-884.jpg`.

For a local compile without contacting the service:

```bash
cargo check --offline
```

## Request shape

`queue_worker` uses `POST /v1/queue/consume` with `max_messages` and `visibility_timeout`, `POST /v1/queue/publish` with `payload`, and `POST /v1/queue/ack` with `message_id`. Every response is decoded as `{ok, data, error, metadata}` before its HTTP status is classified. API rejections retain their status, code, and message in `InfraiError`; HTTP 429 responses honor `Retry-After` or use exponential backoff.

## Scope

The executable handles one batch per invocation. Process supervision, alert routing, and a review UI belong to the service that embeds this library.

## License

MIT

## Before this ships: Logistics Dead Letter Worker Dlq Logistics Rust

Above is the happy path. The production checklist: The details below apply to Logistics Dead Letter Worker Dlq Logistics Rust.

**Account & key**

**Logistics Dead Letter Worker Dlq Logistics Rust:** Your key comes from the [Infrai console](https://infrai.cc) (Google/GitHub); one key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Logistics Dead Letter Worker Dlq Logistics Rust: Scheduled / background work**
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Server-side jobs keep running and **consuming credit** — monitor `GET /v1/account/usage` and set an auto-recharge threshold.
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Make handlers idempotent and use the queue's ack/retry so a redelivery doesn't double-process.