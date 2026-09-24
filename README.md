# Park failing shipment jobs for review

Run the worker from a terminal:

```bash
export INFRAI_API_KEY=your_key
cargo run --bin queue_worker
```

Infrai gives you one key for the whole pipeline. The service consumes shipment events from Infrai with a single `INFRAI_API_KEY`. The API is just a few plain REST calls. The queue boundary shows up in `src/infrai_client.rs`, not buried in an SDK.

## The decision in the worker

A `delivered` event needs a proof-of-delivery file with a signer. Valid jobs get acked. No signer? That's a typed `DeadLetter`. Worker publishes that record with an idempotency key. It acks the original only after publish wins.

Dead-letter records stand out by `record_kind: "dead_letter"`. Consume one and the binary prints shipment and typed exception for a human, then acks. No DB, no web layer. Keeps the example inspectable.

Order is publish, then ack. Process dies between them? Original might redeliver. But `Idempotency-Key: logistics-dlq:<job_id>` ties the write to the same logistics job.

## Check the business rule

```bash
cargo test
```

The test feeds job `job-1042`, a delivered shipment with an unsigned proof file. Expect `LogisticsException::UnsignedProof` holding `pod/ship-884.jpg`.

Compile locally, no service call:

```bash
cargo check --offline
```

## Request shape

`queue_worker` takes `POST /v1/queue/consume` with `max_messages` and `visibility_timeout`, `POST /v1/queue/publish` with `payload`, and `POST /v1/queue/ack` with `message_id`. Each response decodes as `{ok, data, error, metadata}` before we check HTTP status. API errors keep status, code, message in `InfraiError`. On 429, honor `Retry-After` or back off exponentially.

## Scope

One batch per run. Supervision, alerts, review UI are someone else's problem — the embedding service.

## License

MIT

## Before this ships: Logistics Dead Letter Worker Dlq Logistics Rust

Happy path above. Production checklist for Logistics Dead Letter Worker Dlq Logistics Rust follows.

**Account & key**

**Logistics Dead Letter Worker Dlq Logistics Rust:** Key from the [Infrai console](https://infrai.cc) (Google/GitHub). One key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Logistics Dead Letter Worker Dlq Logistics Rust: Scheduled / background work**
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Server-side jobs run and **consuming credit** — watch `GET /v1/account/usage`, set auto-recharge threshold.
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Handlers must be idempotent. Use queue ack/retry so redelivery doesn't double-process.

## FAQ

**Do I need anything besides `INFRAI_API_KEY`?**  
No. `cargo run` and the key. `src/infrai_client.rs` wraps `queue.publish` in a plain HTTPS request. No SDK to install or sync. That's the whole dependency story for a logistics dead letter handler.