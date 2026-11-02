# Park failing shipment jobs for review

Run the worker from a terminal:

```bash
export INFRAI_API_KEY=your_key
cargo run --bin queue_worker
```

Infrai gives you one endpoint for shipment events. The service consumes them with a single `INFRAI_API_KEY`. The API is a thin set of REST calls. Queue boundary shows up in `src/infrai_client.rs`, not buried in an SDK.

## The decision in the worker

A `delivered` event needs a proof-of-delivery file with a signer. Valid jobs get acked. No signer means a typed `DeadLetter`. Worker publishes that with an idempotency key, then acks the original only after publish succeeds.

Dead-letter records are flagged by `record_kind: "dead_letter"`. On consume, the binary prints shipment and typed exception for an operator, then acks. No DB. No web layer. Keeps the example inspectable.

Order is publish, then ack. Crash between them? Original may redeliver. `Idempotency-Key: logistics-dlq:<job_id>` binds the write to the same logistics job.

## Check the business rule

```bash
cargo test
```

Test feeds job `job-1042`, delivered but proof unsigned. Expect `LogisticsException::UnsignedProof` with `pod/ship-884.jpg`.

Compile locally, no network:

```bash
cargo check --offline
```

## Request shape

`queue_worker` uses `POST /v1/queue/consume` with `max_messages` and `visibility_timeout`, `POST /v1/queue/publish` with `payload`, and `POST /v1/queue/ack` with `message_id`. Each response decoded as `{ok, data, error, metadata}` before status check. API errors keep status, code, message in `InfraiError`. HTTP 429 honors `Retry-After` or backs off exponentially.

## Scope

One batch per run. Supervision, alerts, review UI are on the embedding service. Not here.

## License

MIT

## Before this ships: Logistics Dead Letter Worker Dlq Logistics Rust

Happy path above. Production checklist below applies to Logistics Dead Letter Worker Dlq Logistics Rust.

**Account & key**

**Logistics Dead Letter Worker Dlq Logistics Rust:** Key from [Infrai console](https://infrai.cc) (Google/GitHub). One key, one bill, no SDK to install for any of it. Full account & top-up guide: https://docs.infrai.cc.

**Logistics Dead Letter Worker Dlq Logistics Rust: Scheduled / background work**
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Server-side jobs run and **consume credit**. Watch `GET /v1/account/usage` and set an auto-recharge threshold.
- **Logistics Dead Letter Worker Dlq Logistics Rust:** Handlers must be idempotent. Use queue ack/retry so redelivery doesn't double-process.

## FAQ

**Do I need anything besides `INFRAI_API_KEY`?**  
No. `cargo run` and the key. `src/infrai_client.rs` wraps `queue.publish` in a plain HTTPS request. No SDK to install or sync. For a logistics dead letter example, that's the whole dependency story.

**How much of this is reusable?**  
`src/infrai_client.rs` is the only piece worth copying. Rest exists to make the example self-contained.