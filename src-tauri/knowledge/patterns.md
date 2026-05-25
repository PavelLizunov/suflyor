# Patterns Reference v1

System design + architectural + algorithm patterns. Each entry: `## <name>` heading, body with structure / when-to-use / tradeoffs.
Parser: split on `\n## `.

## url-shortener
**Problem:** bit.ly clone. tinyurl.com / 1B+ URLs / sub-100ms read.
**Approach:**
- ID generation: Base62 of monotonic counter (~7 chars for 3.5T). Counter via Redis INCR or Snowflake.
- Storage: KV store (DynamoDB / Cassandra). Key = short ID, value = long URL + metadata.
- Cache hot URLs in Redis (top 1% = 50% traffic). TTL 1h.
- Reads >> writes (1000:1). Replicate read side aggressively.
**Tradeoffs:** Monotonic counter centralizes coordination; can use base62-of-hash for distributed gen at cost of collision check. Custom aliases need uniqueness check on write path.

## twitter-feed
**Problem:** 300M users, each follows N people, fetch their timeline in <200ms.
**Approach:**
- **Fan-out on write** (push): when user X tweets, write to N follower feeds. Fast reads, expensive for celebrities (10M followers = 10M writes per tweet).
- **Fan-out on read** (pull): on feed view, query tweets from followed users + merge. Slow reads.
- **Hybrid:** push for normal users, pull for celebrities. Detect at user level.
- Storage: feed cache in Redis sorted set (timestamp = score), tweet content in main DB.
**Tradeoffs:** Push optimizes 99% of users at cost of celeb edge case. Hybrid adds complexity but bounds worst case.

## chat-system
**Problem:** WhatsApp/Slack. Real-time messaging, history, group chats, presence.
**Approach:**
- WebSocket per user (long-lived). Connection layer separate from logic layer.
- Message → write to DB → push to recipients via their connections (lookup user → conn server in Redis).
- History: Cassandra (write-heavy, append). Latest N messages cached.
- Group chat: store one copy, fan out to members on send. Member list cached.
- Presence: Redis with TTL refresh on heartbeat.
**Tradeoffs:** Push delivery cheaper than poll. End-to-end encryption shifts content opacity but breaks server-side moderation.

## news-feed-ranking
**Problem:** Facebook-style relevance-ranked feed.
**Approach:**
- Candidate generation: stream of potentially interesting items (recent posts, friends, followed pages).
- Ranking: ML model scores each candidate (predicted engagement). Trained on click/like/share history.
- Diversity: penalize over-representation (one author dominating).
- Time decay: older content scored lower.
- Filter: hide already-seen, blocked content, spam.
**Tradeoffs:** Real-time scoring expensive — pre-rank during off-peak, refresh hourly. Cold start (new user) — fallback to chronological + popular.

## search-engine
**Problem:** Google-scale search index.
**Approach:**
- Crawl: web crawlers fetch pages, respect robots.txt, frontier queue.
- Parse: extract text, links, metadata.
- Index: inverted index (term → list of doc IDs with positions). Sharded by term hash or document.
- Rank: PageRank (link analysis) + text relevance (TF-IDF, BM25) + ML signals.
- Serve: query → fanout to all shards → merge top-K → return.
**Tradeoffs:** Index size huge — segment + compress. Real-time index updates vs batch — most search engines have separate hot/cold tiers.

## payment-system
**Problem:** Stripe-style payment processing.
**Approach:**
- Idempotency: client provides Idempotency-Key, server dedupes retries.
- Authorization vs capture: 2-step for cards (auth holds funds, capture commits later).
- Webhook for async results (refund completed, chargeback received). Sign webhooks for trust.
- Reconciliation: nightly job compares our records vs bank statements.
- Compliance: PCI DSS — never store full card numbers (only last 4 + tokenized).
**Tradeoffs:** Strong consistency on balance. Eventual consistency on reporting. Audit log immutable.

## ad-click-counter
**Problem:** Count ad clicks at billions/day scale, near-real-time.
**Approach:**
- Ingest: HTTPS endpoint → Kafka topic per shard.
- Stream processing: Flink/Spark counts per (ad_id, hour) — sliding window.
- Output: write to OLAP DB (ClickHouse) for queries.
- Hot path: Redis HyperLogLog for unique users.
- Cold path: hourly batch over Kafka archive for accuracy.
**Tradeoffs:** Stream vs batch trade latency for accuracy. Fraud detection adds layers (rate limit per IP, ML).

## rate-limiter
**Problem:** Limit API requests per user/IP.
**Algorithms:**
- **Token bucket:** N tokens, refill rate R, each request takes 1. Allows bursts.
- **Leaky bucket:** fixed output rate, requests queue. Smooths bursts, drops on overflow.
- **Fixed window:** count per discrete window. Simple, 2× burst at boundary.
- **Sliding window:** count in last N seconds (log of timestamps). Most accurate, most memory.
- **Sliding log:** keep all timestamps. Most memory.
**Approach:** Redis with Lua script for atomic increment + check. Or distributed cache like Hazelcast.
**Tradeoffs:** Token bucket = good UX (bursts ok). Fixed window = cheapest but bad at boundary.

## cache-aside
**Pattern:** App checks cache → miss → reads DB → populates cache → returns.
**Pros:** Simple, app controls. Fault-tolerant (cache down → still works, slower).
**Cons:** Stale data possible. Cache stampede on hot key (many requests miss simultaneously).
**Mitigations:** Probabilistic early refresh. Lock + double-check pattern.

## read-through
**Pattern:** Cache provider sits in front of DB. App only talks to cache. Cache fetches DB on miss.
**Pros:** Cleaner app code. Cache layer handles concurrency.
**Cons:** Cache becomes SPOF. Less flexible than cache-aside.

## write-through
**Pattern:** Write goes to cache AND DB synchronously.
**Pros:** Cache always fresh.
**Cons:** Slow writes (2× latency). Wasted cache space for write-only data.

## write-behind
**Pattern:** Write to cache, async flush to DB.
**Pros:** Fast writes.
**Cons:** Data loss risk on cache crash. Eventual consistency.

## write-around
**Pattern:** Write to DB only. Cache misses on next read.
**Pros:** Cache holds only frequently-read data.
**Cons:** Read-after-write returns from DB (slower).

## sharding-by-key
**Pattern:** `shard = hash(key) % N`.
**Pros:** Even distribution. Simple lookup.
**Cons:** N change → most keys move (use consistent hashing). Multi-key ops cross shards.

## sharding-by-range
**Pattern:** Partition by key range (e.g. A-M shard 1, N-Z shard 2).
**Pros:** Range queries efficient.
**Cons:** Hotspots (recent data shard hot in time-series).

## consistent-hashing
**Pattern:** Hash ring of nodes. Each key goes to next clockwise node. Adding/removing node moves 1/N keys.
**Use cases:** Caches (memcached, Cassandra), CDNs, gateways routing to backends.
**Variant:** Bounded-load — limit per node capacity to avoid skew.

## leader-follower
**Pattern:** One node accepts writes (leader), others replicate (followers).
**Pros:** Simple consistency model.
**Cons:** Leader = bottleneck for writes. Failover takes time.
**Examples:** PostgreSQL streaming replication, MySQL replication.

## multi-leader
**Pattern:** Multiple nodes accept writes. Replicate to each other.
**Pros:** Local writes in multi-region. Higher write throughput.
**Cons:** Conflict resolution needed (LWW, CRDTs).
**Examples:** Cassandra, CockroachDB (with caveats), CouchDB.

## leaderless
**Pattern:** Any node accepts writes. Writes go to N replicas, reads from M (quorum).
**Pros:** No single point of failure. Tunable consistency.
**Cons:** Read repair / anti-entropy needed. Eventual consistency.
**Examples:** Cassandra, DynamoDB, Riak.

## quorum
**Pattern:** Operations require majority. W + R > N for strong consistency (with N total replicas).
**Variants:** Majority quorum (N/2+1), sloppy quorum (relax during partition).

## two-phase-commit — 2pc
**Pattern:** Coordinator says "prepare", participants vote, coordinator commits if all yes.
**Pros:** Strong consistency across systems.
**Cons:** Blocking. Coordinator SPOF. Slow.
**Modern alternative:** Saga pattern (compensation-based).

## saga
**Pattern:** Long-running transaction as sequence of local transactions, each with compensating action on failure.
**Variants:**
- **Choreography:** services react to events
- **Orchestration:** central coordinator (Temporal, Camunda)
**See snippet /saga.**

## event-sourcing
**Pattern:** Store events (state changes), not current state. Current state = fold(events).
**Pros:** Audit log built-in. Time travel. Reactive views via projections.
**Cons:** Storage grows forever (snapshots help). Schema evolution complex.

## cqrs
**Pattern:** Separate Command (write) and Query (read) models. Different schemas, different DBs.
**Pros:** Optimize each independently. Scale reads + writes separately.
**Cons:** Eventual consistency between sides. Complexity.

## outbox-pattern
**Pattern:** DB transaction writes business data + event row in `outbox` table atomically. Separate process reads outbox + publishes to broker.
**Pros:** Atomicity of "DB write + event publish".
**Cons:** Extra process. At-least-once delivery (dedup downstream).

## inbox-pattern
**Pattern:** On receive, write event to `inbox` table within local transaction with business changes. Dedup via event ID.
**Pros:** Exactly-once processing.
**Cons:** Storage overhead.

## change-data-capture — cdc
**Pattern:** Capture DB changes (binlog / WAL) → publish to broker → consumers update derived systems.
**Tools:** Debezium, AWS DMS, Maxwell.
**Use cases:** sync to search index, data warehouse, cache invalidation.

## materialized-view
**Pattern:** Precomputed query result, refreshed on schedule or trigger.
**Pros:** Fast queries.
**Cons:** Storage cost. Staleness.
**Variants:** PostgreSQL materialized views, ClickHouse continuous aggregates.

## bloom-filter-pattern
**Pattern:** Check membership cheaply before expensive lookup.
**Use cases:** Cache (skip DB if not in filter), web crawler (have I seen this URL?), Bigtable rows.

## bulkhead
**Pattern:** Isolate resources per upstream. Failure in one doesn't drain pool for others.
**Implementation:** Separate thread pools, separate connection pools.
**Goal:** Limit blast radius.

## circuit-breaker
**Pattern:** Open circuit after N failures, fail fast, periodically half-open to test.
**States:** Closed → Open → Half-Open → Closed/Open.
**See snippet /circuit.**

## retry-with-backoff
**Pattern:** On failure, wait + retry. Exponential backoff (1s, 2s, 4s, 8s) + jitter.
**Anti-pattern:** Tight retry loops causing thundering herd.

## timeout-cascade
**Pattern:** Each tier's timeout should be longer than sum of downstream timeouts.
**Example:** Client 30s > LB 25s > App 20s > DB 10s.

## deadline-propagation
**Pattern:** Pass deadline through call chain. Downstream services know how long left.
**Implementation:** gRPC `context.WithDeadline`, HTTP `X-Request-Deadline` header.

## fan-out-aggregator
**Pattern:** One request fans out to N parallel calls, aggregate results.
**Variants:** Wait for all (slow). Wait for K of N (faster, partial). Hedge (start 2, take faster).

## hedged-requests
**Pattern:** Send same request to multiple replicas, use first response.
**Pros:** Tail latency improved.
**Cons:** More load. Implement cancellation of losers.

## load-shedding
**Pattern:** Drop requests when overloaded. Better than slow death.
**Strategies:** Priority-based, FIFO drop tail, oldest-first drop.

## graceful-degradation
**Pattern:** Reduce features under stress vs full outage.
**Examples:** Disable recommendations when overloaded but keep core. Show stale cached data.

## sidecar
**Pattern:** Companion container in same Pod for cross-cutting concerns (proxy, logging, metrics).
**Examples:** Envoy in Istio mesh, Fluent Bit log shipper.

## ambassador
**Pattern:** Sidecar acting as proxy for external services. App talks to localhost.
**Use cases:** TLS termination, retry logic, service discovery.

## adapter
**Pattern:** Sidecar that normalizes data format for app.
**Use cases:** Old app emits weird logs → adapter transforms to standard format.

## strangler-fig
**Pattern:** Gradually replace old system. Route by feature/endpoint. New features in new system. Old system shrinks.
**Pros:** Incremental risk. No big-bang cutover.

## branch-by-abstraction
**Pattern:** Introduce abstraction layer → swap implementations → remove old.
**Use cases:** Database migration, framework swap.

## feature-flag
**Pattern:** Code path enabled/disabled at runtime. Decouple deploy from release.
**Use cases:** A/B tests, kill switches, gradual rollout, per-customer toggles.

## blue-green-deploy
**Pattern:** Two identical envs (blue/green). Deploy to idle. Switch traffic atomically. Old idle ready for rollback.
**See snippet /deploy.**

## canary-deploy
**Pattern:** Send small % of traffic to new version. Monitor. Scale up if good, abort if bad.
**Tools:** Argo Rollouts, Flagger.
**See snippet /deploy.**

## rolling-deploy
**Pattern:** Replace instances gradually. K8s Deployment default. `maxSurge` + `maxUnavailable`.
**Cons:** Mixed traffic during rollout. Hard rollback (need reverse rolling).

## dark-launch
**Pattern:** Deploy code that runs but isn't visible. Validate perf, gather logs.
**Use cases:** Validate new ML model offline before exposing.

## shadow-traffic
**Pattern:** Send copy of prod traffic to new version. Compare results offline.
**Pros:** Real-traffic testing without user impact.
**Cons:** Side effects (writes to DB) must be isolated.

## chaos-engineering
**Pattern:** Inject failures in prod to validate resilience.
**Tools:** Chaos Monkey, Gremlin, Chaos Mesh, Litmus.
**Discipline:** Steady state hypothesis → vary variable → measure → expand blast radius.

## game-day
**Pattern:** Scheduled chaos exercise. Team practices response.
**Outcome:** Found gaps in runbook, observability, alerts.

## throttling
**Pattern:** Server-side rate limit enforcement. Returns 429.
**Headers:** `X-RateLimit-Limit`, `X-RateLimit-Remaining`, `X-RateLimit-Reset`.

## backpressure
**Pattern:** Downstream signals upstream to slow down.
**Implementation:** Bounded queues (drop or block), gRPC flow control, reactive streams.

## queue-based-load-leveling
**Pattern:** Insert queue between client and service. Service consumes at sustainable rate. Burst absorbed by queue.
**Tradeoff:** Adds latency. Queue can grow unbounded → monitor + cap.

## competing-consumers
**Pattern:** Multiple consumers read from same queue. Each message handled by one.
**Scale:** Add consumers to scale.
**Examples:** Kafka consumer groups, SQS multiple workers.

## publisher-subscriber
**Pattern:** Publisher emits events. Each subscriber gets a copy.
**Examples:** Redis pub/sub, Kafka with separate consumer groups, SNS.

## priority-queue
**Pattern:** Higher-priority messages processed first.
**Implementations:** Separate queues per priority + weighted consumer, RabbitMQ priority queues.

## dead-letter-queue — dlq
**Pattern:** Messages that failed N processing attempts go to DLQ. Manual triage.
**Examples:** SQS DLQ, Kafka separate topic.

## claim-check
**Pattern:** Don't send large payloads via message bus. Upload to S3, send pointer in message.
**Use cases:** Image processing pipeline, ML model inputs.

## scheduler-agent-supervisor
**Pattern:** Scheduler distributes work. Agents execute. Supervisor monitors + restarts failures.
**Examples:** Kubernetes (scheduler + kubelet + controller manager), Airflow.

## leader-election-pattern
**Pattern:** Multiple nodes, one elected leader for coordination role.
**Implementations:** Raft, Paxos, ZooKeeper ephemeral nodes, K8s coordination.k8s.io leases.

## fencing-token
**Pattern:** Leader includes monotonic token in all writes. Replicas reject lower tokens.
**Goal:** Prevent old leader (zombie) from corrupting state after failover.

## gossip-protocol
**Pattern:** Nodes randomly exchange state with peers. Information spreads epidemically.
**Examples:** Cassandra (node membership), Serf, Consul.

## anti-entropy
**Pattern:** Periodic reconciliation between replicas to fix divergence.
**Examples:** Cassandra repair, DynamoDB Merkle tree comparison.

## merkle-tree
**Pattern:** Hash tree where each parent = hash of children. Compare top hash to detect any diff. Drill down to find specific diff.
**Use cases:** Git, BitTorrent, Cassandra anti-entropy.

## crdt-counter
**Pattern:** Conflict-free Replicated Data Type. Each node increments local counter. Merge = sum.
**Examples:** Riak counters, Redis CRDTs.

## crdt-set
**Pattern:** OR-Set (add wins) or LWW-Set (last write wins). Merge by union with conflict rules.

## eventual-consistency
**Pattern:** No coordination on write. Replicas converge eventually.
**Pros:** High availability. Low write latency.
**Cons:** Stale reads possible. App must handle.

## strong-consistency
**Pattern:** All readers see latest write.
**Implementations:** Single-leader with sync replication, consensus protocols.
**Trade:** Throughput + availability.

## read-your-writes
**Pattern:** Client sees their own writes immediately, even if other clients see eventual consistency.
**Implementation:** Stick session to replica that has writes. Or version vectors.

## monotonic-reads
**Pattern:** Subsequent reads see same or newer data, never older.
**Failure case:** Read from replica A (new), then replica B (old) — saw timeline go backward.

## monotonic-writes
**Pattern:** Writes from same client preserved in order.

## causal-consistency
**Pattern:** Causally related operations preserved. Concurrent ops may be reordered.
**Stronger than:** eventual consistency.
**Weaker than:** strong consistency.

## linearizability
**Pattern:** Operations appear instantaneous in some real-time order. Strongest.
**Cost:** Coordination. Limits scalability.

## serializability
**Pattern:** Transactions appear as if executed serially. About transaction isolation, not single ops.

## map-reduce
**Pattern:** Map: parallel processing. Shuffle: group by key. Reduce: aggregate per key.
**Examples:** Hadoop MapReduce, Spark, Flink.

## lambda-architecture
**Pattern:** Batch layer (accurate) + speed layer (real-time) + serving layer (merged).
**Cons:** Maintain two pipelines (bug compatible).

## kappa-architecture
**Pattern:** Only stream processing. Replay from log for backfill.
**Pros:** Single codebase.
**Examples:** Kafka + Flink/Spark Streaming.

## medallion-architecture
**Pattern:** Bronze (raw) → Silver (cleaned) → Gold (aggregated business metrics) data layers.
**Use cases:** Data lake organization.

## elt-vs-etl
**ETL:** Extract → Transform → Load. Transform in middle tier.
**ELT:** Extract → Load → Transform. Transform in warehouse (SQL). Modern preferred.

## data-mesh
**Pattern:** Domain-owned data products. Decentralized data ownership. Replaces central data team.

## medallion-bronze
Raw immutable data, schema-on-read.

## medallion-silver
Cleaned, joined, deduplicated.

## medallion-gold
Aggregated business-ready datasets.

## hexagonal-architecture
**Pattern:** Domain core in center. Ports (interfaces) define interactions. Adapters implement ports for specific tech (DB, HTTP, gRPC).
**Pros:** Domain testable in isolation. Swap infra easily.

## clean-architecture
**Pattern:** Concentric layers. Inner = pure business logic. Outer = frameworks. Dependencies point inward only.
**Aka:** Onion architecture (similar).

## ddd-bounded-context
**Pattern:** Explicit boundary for a model. Within: one ubiquitous language. Across: translation layers.
**Maps to:** Microservice boundary (ideal).

## ddd-aggregate
**Pattern:** Cluster of related objects treated as one unit. Single entry point (aggregate root). Transactional boundary.

## anti-corruption-layer
**Pattern:** Translator between bounded contexts. Prevents foreign domain concepts leaking into yours.

## api-gateway
**Pattern:** Single entry point for all client requests. Routes to backend services.
**Features:** Auth, rate limit, request transformation, response aggregation.
**Examples:** AWS API Gateway, Kong, Apigee, Envoy.

## backend-for-frontend — bff
**Pattern:** Separate gateway per client type (web BFF, mobile BFF). Tailored data shape.
**Avoids:** One gateway trying to please all clients.

## service-mesh
**Pattern:** Sidecar proxies handle service-to-service concerns (mTLS, retries, observability).
**Examples:** Istio, Linkerd. See snippet /mesh.

## service-discovery
**Pattern:** Services register their location. Clients query registry for routes.
**Examples:** Consul, etcd, K8s DNS + Service abstraction.

## client-side-load-balancing
**Pattern:** Client picks backend (vs LB picking). No SPOF. Latency-aware.
**Examples:** gRPC client-side, K8s Service via kube-proxy.

## server-side-load-balancing
**Pattern:** Client talks to LB. LB picks backend.
**Examples:** AWS ALB, Nginx upstream.

## ringpop
**Pattern:** Cooperative gossip-based clustering. Used by Uber for stateful services routing.

## raft-consensus
**Pattern:** Leader election + log replication. Simpler than Paxos. Used by etcd, Consul, TiKV.

## paxos
**Pattern:** Original consensus algorithm. Harder to implement than Raft.

## zab — ZooKeeper Atomic Broadcast
**Pattern:** ZooKeeper's consensus. Similar to Paxos with primary order.

## fenced-leader
**Pattern:** Leader has lease with timestamp. Replicas reject writes with older timestamps.

## leases
**Pattern:** Time-bounded lock. Auto-expires if not renewed. Avoids stuck locks.
**Examples:** ZooKeeper ephemeral nodes, K8s coordination leases, Etcd leases.

## vector-clock-pattern
**Pattern:** Each node tracks counter, exchanges with peers on operations. Compare vectors to determine causality.
**Use cases:** DynamoDB conflict detection, Riak.

## tombstone
**Pattern:** Delete = write special "deleted" marker. Real removal happens during compaction.
**Use cases:** Cassandra, distributed file systems.

## compaction
**Pattern:** Background process merges + cleans up data files. Removes tombstones, deduplicates.
**Examples:** Kafka log compaction, LSM tree (Cassandra, RocksDB), Postgres VACUUM.

## lsm-tree
**Pattern:** Log-Structured Merge tree. Writes to in-memory buffer (memtable), flushed to immutable SSTables, periodically merged.
**Pros:** Write-optimized.
**Cons:** Read amplification (check multiple SSTables).
**Examples:** Cassandra, RocksDB, LevelDB.

## b-tree-storage
**Pattern:** Balanced tree. Pages stored on disk. Updates in-place.
**Pros:** Read-optimized.
**Cons:** Write amplification.
**Examples:** PostgreSQL, MySQL InnoDB, MongoDB WiredTiger.

## copy-on-write
**Pattern:** Modifying data creates new copy. Original unchanged.
**Use cases:** ZFS, Btrfs, immutable infrastructure, fork() semantics.

## append-only-log
**Pattern:** Writes appended to end. Never modify in place.
**Pros:** Simple, replay-able, audit trail.
**Examples:** Kafka, Postgres WAL, Git objects.

## hash-partitioning
**Pattern:** Partition by hash(key). Uniform distribution. Bad for range scans.

## range-partitioning
**Pattern:** Partition by key range. Range scans efficient. Hotspots possible.

## composite-partitioning
**Pattern:** Combine multiple strategies. Hash for distribution + range within partition for queries.
**Examples:** Cassandra (partition key + clustering key), ClickHouse (PARTITION BY + ORDER BY).

## reshard
**Pattern:** Split or merge partitions as load grows. Hard. Plan from start.
**Approaches:** Dynamic sharding (Vitess), pre-split + virtual partitions.

## tiered-storage
**Pattern:** Hot data in fast tier (SSD/RAM), cold in slow (S3, tape).
**Examples:** Kafka tiered storage, S3 lifecycle, ClickHouse cold disks.

## index-only-scan
**Pattern:** Query satisfied entirely by index, no table lookup. Covered query.
**Optimization:** Include needed columns in index even if not searched.

## partial-index
**Pattern:** Index only rows matching a WHERE clause. Smaller, faster.
**Examples:** `CREATE INDEX ON users(email) WHERE active = true`.

## expression-index
**Pattern:** Index on computed value, not raw column.
**Examples:** `CREATE INDEX ON users(LOWER(email))` for case-insensitive search.

## fk-vs-no-fk
**FK pros:** Referential integrity. Cascade deletes.
**FK cons:** Lock contention. Migration pain.
**At scale:** Often dropped in favor of app-level checks.

## natural-key
**Pattern:** Use business identifier as PK (e.g. email).
**Cons:** Changes are painful.
**Recommendation:** Use surrogate key (UUID/serial) + unique index on natural key.

## surrogate-key
**Pattern:** System-generated PK (auto-increment, UUID).
**Pros:** Stable, opaque to business.
**Cons:** Extra index on natural key for lookups.

## ulid
**Pattern:** Universally Unique Lexicographically Sortable ID. 128-bit, sorts by time prefix.
**Pros:** Better than UUID v4 for indexed inserts (less B-tree fragmentation).

## snowflake-id
**Pattern:** 64-bit ID = timestamp + worker ID + sequence. Twitter's ID generator.
**Pros:** Sortable, distributed gen.

## uuid-v4-vs-v7
**v4:** Random. Random insertion = B-tree fragmentation.
**v7:** Time-ordered (2024 RFC). Better DB performance.

## reverse-proxy-pattern
**Pattern:** Proxy in front of services. Handles TLS, caching, rate limiting, A/B routing.
**Examples:** nginx, HAProxy, Envoy, Caddy.

## sidecar-proxy
**Pattern:** Per-pod proxy. Service mesh data plane.
**Examples:** Envoy in Istio, linkerd2-proxy in Linkerd.

## ingress-pattern
**Pattern:** Cluster entry point for HTTP/HTTPS. Hostname/path routing.
**Implementations:** nginx-ingress, Traefik, Contour, AWS ALB Ingress Controller.

## sticky-session-pattern
**Pattern:** Pin client to backend for session affinity.
**Avoid if possible:** Stateless app + Redis session > sticky.

## sharding-by-customer
**Pattern:** Each customer's data on dedicated shard. "Pod" model.
**Pros:** Isolation. Big customers can have own shard.
**Cons:** Imbalanced load.

## multi-tenant-data-model
**Patterns:**
- Shared DB, shared schema (tenant_id column)
- Shared DB, separate schema per tenant
- Separate DB per tenant (highest isolation)
**Tradeoff:** Isolation vs operational cost.

## row-level-security
**Pattern:** DB enforces tenant_id filter. Apps can't accidentally leak across tenants.
**Examples:** PostgreSQL RLS, Spanner.

## token-based-auth
**Pattern:** Client sends token (JWT, opaque) in header. Server validates.
**Pros:** Stateless. Works across services.
**Cons:** Revocation harder.

## session-based-auth
**Pattern:** Server stores session, client sends session ID (cookie).
**Pros:** Easy to revoke.
**Cons:** Server state. Sticky sessions or shared store.

## refresh-token-rotation
**Pattern:** Refresh token used → new refresh issued, old invalidated. Reuse detection = breach.

## oauth2-token-introspection
**Pattern:** Resource server calls auth server to validate token (vs local JWT verify).
**Pros:** Real-time revocation.
**Cons:** Latency, dependency.

## mtls-pattern
**Pattern:** Both client and server present certs. Mutual auth.
**Use cases:** Service-to-service in mesh, B2B APIs.

## zero-trust-pattern
**Pattern:** Never trust based on network location. Always verify identity + posture.
**Implementation:** mTLS, identity-aware proxies, just-in-time access.

## secrets-rotation
**Pattern:** Frequently change credentials. Minimize blast radius of leaked secret.
**Tools:** Vault dynamic secrets, AWS Secrets Manager rotation.

## envelope-encryption
**Pattern:** Encrypt data with DEK (Data Encryption Key). Encrypt DEK with KEK (Key Encryption Key) from KMS.
**Pros:** Fast bulk encryption + safe key storage.

## tokenization
**Pattern:** Replace sensitive data with token. Original stored in secure vault.
**Use cases:** PCI compliance (replace card # with token).

## data-masking
**Pattern:** Replace sensitive fields in non-prod environments (dev, staging).
**Tools:** Postgres anon, custom ETL.

## anonymization
**Pattern:** Remove or generalize PII so individual can't be identified.
**Caution:** k-anonymity often insufficient. Differential privacy stronger.

## differential-privacy
**Pattern:** Add calibrated noise to outputs. Mathematical guarantee individual contribution hidden.
**Used by:** Apple, Google, US Census.

## federated-learning-pattern
**Pattern:** Train ML model across devices without centralizing data. Model updates aggregated.
**Use cases:** Mobile keyboard predictions, healthcare.

## homomorphic-encryption
**Pattern:** Compute on encrypted data without decrypting.
**Pros:** Privacy-preserving.
**Cons:** Very slow (still impractical for most cases).

## multi-party-computation — mpc
**Pattern:** Multiple parties jointly compute on private inputs without revealing them.
**Use cases:** Threshold signatures, private set intersection.

## sharded-counter
**Pattern:** N counters, each writer picks random shard. Read = sum all shards.
**Pros:** Avoids write hotspot.
**Examples:** Google Analytics counters, Stripe-style metrics.

## lossy-counting
**Pattern:** Approximate frequency counting over stream. Sublinear memory.
**Algorithms:** Count-Min Sketch, Space-Saving.

## time-series-downsampling
**Pattern:** Old data resolution reduced. e.g. last hour at 1s, last week at 1m, older at 1h.
**Tools:** Prometheus recording rules, RRDtool, InfluxDB downsampling.

## time-series-rollup
**Pattern:** Pre-aggregate metrics over windows for fast queries.
**Examples:** Datadog rollups, Prometheus recording rules.

## hot-warm-cold-storage
**Pattern:** Tiered storage by access frequency. Hot = SSD recent, warm = HDD, cold = object store.
**Examples:** Elasticsearch ILM, ClickHouse storage policies.

## leaderboard
**Pattern:** Top N by score. Real-time updates.
**Implementation:** Redis sorted set (`ZADD`, `ZRANGE`, `ZREVRANGE`). O(log N).

## recommendation
**Patterns:**
- Collaborative filtering (user-user, item-item).
- Content-based (similar items).
- Matrix factorization (SVD, ALS).
- Deep learning (embeddings, two-tower models).

## semantic-search
**Pattern:** Embed query + docs in vector space. Find nearest neighbors.
**Tools:** pgvector, Pinecone, Weaviate, Qdrant.

## hybrid-search
**Pattern:** Combine keyword search (BM25) + semantic (vector). Reciprocal rank fusion.
**Pros:** Better than either alone.

## chunking-strategy
**Pattern:** Split docs for RAG. Trade context vs precision.
**Approaches:** Fixed-size, sentence-based, paragraph, semantic (embeddings).

## retrieval-augmented-generation — rag
**Pattern:** LLM + vector DB. Retrieve relevant docs, include in prompt.
**Pros:** Grounded answers, reduce hallucinations.

## reranker
**Pattern:** After initial retrieval, rerank top-N with a smaller LLM/model for better order.
**Tools:** Cohere Rerank, ColBERT.

## llm-router
**Pattern:** Route query to appropriate model (cheap for simple, expensive for hard).
**Implementation:** Small classifier picks model.

## prompt-chain
**Pattern:** Multi-step LLM calls. Output of one feeds next.
**Examples:** Query → expand → search → summarize → format.

## tool-use-agent
**Pattern:** LLM with function-calling. Decides when to call tools (search, calculator, code exec).
**Loop:** Reason → Act → Observe → Repeat.

## react-agent-pattern
**Pattern:** Reasoning + Acting alternating. Explicit "Thought:" / "Action:" / "Observation:" steps.

## guardrails
**Pattern:** Filter LLM outputs for safety, formatting, policy compliance.
**Tools:** NeMo Guardrails, Guardrails AI, custom regex/classifier.

## prompt-injection-defense
**Pattern:** Treat user input as DATA not instructions. Wrap in clear delimiters. Instruct model to ignore embedded directives.
**Also:** Output filtering for malicious content.

## llm-eval
**Pattern:** Programmatically grade LLM outputs.
**Approaches:** Exact match, BLEU/ROUGE, semantic sim, LLM-as-judge.

## human-in-the-loop
**Pattern:** Critical decisions require human approval. ML suggests, human decides.
**Use cases:** Medical, legal, financial.

## human-feedback-loop
**Pattern:** Capture user corrections, feed into next training cycle.
**Used by:** Search ranking, RLHF.

## a-b-test-pattern
**Pattern:** Random user buckets see different versions. Statistical test for significance.
**Beware:** Multiple comparison correction, novelty effects, network effects.

## multi-armed-bandit
**Pattern:** Online learning across variants. Allocates traffic to winners over time.
**Algorithms:** Epsilon-greedy, UCB, Thompson sampling.

## experimentation-platform
**Pattern:** Centralized A/B test framework. Consistent assignment, metrics, analysis.
**Examples:** Eppo, Statsig, GrowthBook.

## ab-test-pitfalls
- Peeking (early stopping inflates false positives)
- Multiple metrics without correction (Bonferroni or BH-FDR)
- SRM (Sample Ratio Mismatch — buckets aren't 50/50)
- Network effects (your variant affects control's behavior)
- Novelty effect (initial interest decays)

## monolith-to-microservices
**Steps:**
1. Strengthen monolith with tests + observability.
2. Extract first service (best candidate: bounded context with own data).
3. Strangler fig: route by feature.
4. Decouple data (separate DB per service eventually).
5. Repeat for next service.
**Don't:** Big-bang rewrite. Distributed monolith (microservices coupled by shared DB).

## migration-zero-downtime
**Pattern:** Expand → migrate → contract.
1. Add new schema/system (compatible with old).
2. Dual-write to both.
3. Backfill old data.
4. Read from new (validate).
5. Stop dual-write, decommission old.

## big-bang-vs-incremental
**Big-bang:** Replace all at once. High risk. Long planning.
**Incremental:** Strangler fig. Low risk. Long timeline.
**Choose:** Incremental almost always. Exception: technology truly incompatible.

## data-warehouse-design
**Patterns:**
- **Star schema:** Fact table + dimension tables. Simple, fast.
- **Snowflake schema:** Dimensions normalized into sub-dimensions. More normalized.
- **Data vault:** Hubs + links + satellites. Auditable, scalable, complex.

## scd — Slowly Changing Dimension
**Type 1:** Overwrite. No history.
**Type 2:** Add row with version + valid-from/to. Full history.
**Type 3:** Add column for previous value. Limited history.
**Use:** Type 2 usually. Type 1 when history irrelevant.

## fact-vs-dimension
**Fact:** Measurable events (sales, clicks). Numeric.
**Dimension:** Context (customer, product, time). Descriptive.

## kappa-streaming
**Pattern:** Only streaming. Replay from log for backfill.
**Tools:** Kafka + Flink/Spark Structured Streaming.

## checkpoint-restart
**Pattern:** Periodically save state. On failure, restart from last checkpoint.
**Examples:** Spark checkpointing, Flink savepoints.

## exactly-once-semantics
**Pattern:** Each message processed exactly once. Hardest delivery guarantee.
**Implementations:** Kafka transactional API, Flink end-to-end with sinks supporting 2PC.

## at-least-once-semantics
**Pattern:** Each message processed ≥1 time. Possible duplicates.
**Mitigation:** Idempotent consumers (dedup via message ID).

## at-most-once-semantics
**Pattern:** Each message processed ≤1 time. Possible loss.
**Use case:** Metrics where loss tolerable.

## watermark-streaming
**Pattern:** Time-based progress marker in streams. "All events with timestamp ≤ T have arrived". Triggers window closing.

## window-tumbling
**Pattern:** Fixed-size, non-overlapping (e.g. 1-min windows). Simple.

## window-sliding
**Pattern:** Fixed-size, overlapping (e.g. 1-min window every 10s). Multiple aggregations per event.

## window-session
**Pattern:** Variable size based on inactivity gap. "User session" = events with <30min gap.

## algorithm-two-pointers
**Pattern:** Two indices moving through array. Linear time.
**Use cases:** Find pair summing to X, remove duplicates, palindrome check.

## algorithm-sliding-window
**Pattern:** Window expands + contracts over array.
**Use cases:** Max subarray of size K, longest substring with property.

## algorithm-fast-slow-pointer
**Pattern:** One pointer 2× speed of other. Detect cycle in linked list.

## algorithm-binary-search-pattern
**Pattern:** Sorted array → narrow by half each step. O(log n).
**Variants:** First/last occurrence, range search, "binary search on answer" (find min K such that f(K) is true).

## algorithm-dp-knapsack
**Pattern:** Choose items to maximize value within weight constraint. O(n × W).
**Variants:** 0/1 (each item once), unbounded, fractional (greedy works).

## algorithm-dp-lcs
**Pattern:** Longest Common Subsequence. O(n × m) table.
**Generalizes:** diff algorithms, edit distance.

## algorithm-dp-edit-distance
**Pattern:** Levenshtein distance. Min ops (insert/delete/replace) to transform A to B. O(n × m).

## algorithm-dp-coin-change
**Pattern:** Min coins to make amount. Classic DP.
**Variant:** # ways to make amount = different recurrence.

## algorithm-bfs-shortest-path
**Pattern:** BFS on unweighted graph = shortest path. Track visited.

## algorithm-dijkstra-pattern
**Pattern:** Greedy shortest path on non-negative weighted graph. Priority queue. O(E log V).

## algorithm-union-find
**Pattern:** Track connected components. Near-O(1) per op (with path compression + union by rank).
**Use cases:** Kruskal's MST, connectivity queries.

## algorithm-topological-sort
**Pattern:** Order nodes such that all edges point forward. DAG only.
**Algorithms:** Kahn's (BFS-based), DFS-based.
**Use cases:** Build order, course prereqs.

## algorithm-trie-pattern
**Pattern:** Prefix tree. Each path = string.
**Use cases:** Autocomplete, prefix queries, IP routing.

## algorithm-segment-tree
**Pattern:** Tree of intervals. O(log n) range query + point update.
**Use cases:** Range sum/min/max with updates.

## algorithm-fenwick-bit
**Pattern:** Binary Indexed Tree. Like segment tree but simpler implementation.
**Limitation:** Only invertible operations (sum, XOR — not max).

## algorithm-monotonic-stack
**Pattern:** Stack with values in increasing/decreasing order. O(n) for "next greater element" type problems.

## algorithm-monotonic-deque
**Pattern:** Deque variant. Used for sliding window max/min in O(n).

## algorithm-bit-manipulation
**Techniques:**
- `n & (n-1)` — clear lowest set bit
- `n & -n` — isolate lowest set bit  
- `n ^ (n >> 1)` — Gray code
- Popcount via De Bruijn sequences
**Use cases:** Subset enumeration, optimization.

## algorithm-bitmask-dp
**Pattern:** State = bitmask of visited items. 2^n states. O(2^n × n).
**Use cases:** TSP, assignment problems.

## algorithm-sweep-line
**Pattern:** Sort events by coordinate. Process in order, maintaining active set.
**Use cases:** Interval overlap, geometry problems.

## algorithm-meet-in-middle
**Pattern:** Split problem in half, solve each separately, combine.
**Use cases:** Subset sum with N=40 (2^20 × 2^20 vs 2^40).

## algorithm-greedy-pattern
**Pattern:** Local optimal choice each step. Works only when proven correct.
**Examples:** Dijkstra, Kruskal, Huffman, interval scheduling.

## algorithm-divide-conquer
**Pattern:** Split → solve subproblems recursively → combine.
**Examples:** Merge sort, FFT, Karatsuba, Strassen matrix mult.

## algorithm-backtracking
**Pattern:** DFS with pruning. Explore solutions, undo on failure.
**Examples:** N-queens, sudoku, subset sum.

## algorithm-monte-carlo
**Pattern:** Random sampling for approximate answer. Faster than exact.
**Examples:** Monte Carlo tree search (AlphaGo), pi estimation.

## algorithm-las-vegas
**Pattern:** Random algorithm always correct, random runtime.
**Examples:** Randomized quicksort, Knuth shuffle.

## algorithm-amortized-analysis
**Pattern:** Average cost per op over sequence. Some ops expensive but rare.
**Examples:** Dynamic array push O(1) amortized despite O(n) resize.

## algorithm-streaming-percentile
**Pattern:** Estimate quantiles without storing all data.
**Algorithms:** t-digest, HDR histogram, GK summary.

## algorithm-bloom-filter-design
**Pattern:** Bit array + k hash functions. Test membership.
**Tradeoff:** Smaller filter = more false positives. Choose by required FP rate.

## algorithm-count-min-sketch
**Pattern:** 2D array + k hash functions for frequency estimation.
**Use cases:** Heavy hitters in streams.

## algorithm-hyperloglog-design
**Pattern:** Cardinality estimation via leading zeros of hashes.
**Use cases:** Unique visitors, distinct count over big stream.

## algorithm-reservoir-sampling
**Pattern:** Sample k elements from stream of unknown size, each with equal probability.
**Algorithm:** Replace existing sample with prob k/i for i-th element.

## algorithm-skip-list-design
**Pattern:** Probabilistic alternative to balanced trees. Multiple levels of forward pointers.
**Pros:** Simpler than AVL/RB. Used by Redis sorted sets.

## algorithm-rope
**Pattern:** Binary tree of string fragments. O(log n) concat + slice.
**Use cases:** Text editors with huge documents.

## algorithm-suffix-array
**Pattern:** Sorted array of all suffixes. O(n log n) to build, O(m log n) substring search.
**Alternatives:** Suffix tree (more memory).

## algorithm-kmp
**Pattern:** Knuth-Morris-Pratt string match. O(n + m). Preprocessed pattern.

## algorithm-rabin-karp
**Pattern:** Rolling hash substring search. O(n + m) average.
**Use cases:** Multiple pattern search, plagiarism detection.

## algorithm-boyer-moore
**Pattern:** String match scanning right-to-left. Sublinear in best case.

## algorithm-aho-corasick
**Pattern:** Multi-pattern string match. Build trie + failure links.
**Use cases:** Malware signature detection, dictionary matching.

## algorithm-fft
**Pattern:** Fast Fourier Transform. O(n log n) for convolution.
**Use cases:** Signal processing, polynomial multiplication, image compression.

## algorithm-geometry-convex-hull
**Pattern:** Smallest convex polygon containing all points.
**Algorithms:** Graham scan O(n log n), Andrew's monotone chain.

## algorithm-flow-max
**Pattern:** Max flow in network. Ford-Fulkerson, Edmonds-Karp O(VE²).
**Use cases:** Bipartite matching, min cut, assignment.

## algorithm-min-cut
**Pattern:** Minimum edge weight set whose removal disconnects source from sink.
**Max-flow min-cut theorem.**

## algorithm-bipartite-matching
**Pattern:** Match nodes of two sets. Solved by max flow.

## algorithm-lru-design
**Pattern:** HashMap + DoublyLinkedList. O(1) get/put.
**Common interview question.**

## algorithm-lfu-design
**Pattern:** HashMap + frequency buckets + per-bucket DLL. O(1) get/put.
**Trickier than LRU.**

## algorithm-tiny-url-design
**Pattern:** Base62 of monotonic counter. Counter from Redis INCR or Snowflake.

## algorithm-rate-limit-design
**Pattern:** Token bucket via Redis Lua. Atomic check + decrement.
**Distributed:** Each node has portion of budget.

## algorithm-distributed-lock
**Pattern:** Redis `SET key val NX PX 30000`. Renew lease while holding.
**Redlock controversial — single Redis often enough.**

## algorithm-distributed-counter
**Pattern:** Sharded counter to avoid hot key. Read = sum shards.
**For exact counts.**

## algorithm-distributed-id
**Patterns:** Snowflake, ULID, UUID v7. Sortable, distributed gen.

## algorithm-cap-tradeoffs
**For 99.99% availability + global write latency <100ms:** Use AP system (DynamoDB, Cassandra). Accept eventual consistency.
**For strong consistency + bounded latency:** CP system (Spanner, CockroachDB) in single region.

## algorithm-eventual-consistency-design
**Patterns:** Read repair, hinted handoff, anti-entropy, vector clocks, CRDTs.

## algorithm-quorum-design
**N replicas, W writes acked, R reads acked.**
- W + R > N → strong consistency
- W=N → no fault tolerance writes
- R=1, W=1 → highest availability, lowest consistency
**Tunable per query (Cassandra ONE/QUORUM/ALL).**

## algorithm-paxos-design
**Phases:** Prepare (proposer asks acceptors), Accept (proposer sends value), Learn (acceptors notify learners).
**Multi-Paxos:** Stable leader skips prepare phase for stream of values.

## algorithm-raft-design
**Steps:** Leader election → Log replication → Safety (committed entries durable).
**Simpler than Paxos by design.**

## algorithm-merkle-tree-design
**Build:** Hash each leaf, hash pairs, recurse to root.
**Compare:** Root differs → drill down. Find differing leaves in O(log n).
**Use cases:** Git, Cassandra anti-entropy, blockchain.