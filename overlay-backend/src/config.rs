//! User config persisted as JSON in OS data dir.
//! Path: %APPDATA%\overlay-mvp\config.json

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    /// Pre-meeting context (system prompt prefix), free-form.
    /// e.g. "Это собеседование на Senior SRE position. Мой опыт: 7 лет K8s..."
    pub meeting_context: String,

    /// Named profiles for meeting_context (swap quickly).
    pub context_profiles: Vec<ContextProfile>,

    /// Active profile name (matches one of context_profiles[].name), or None.
    pub active_profile: Option<String>,

    /// Audio device names (exact match against WASAPI enumeration).
    pub mic_device: Option<String>, // e.g. "Headset Microphone (A50 Mic)"
    pub system_audio_device: Option<String>, // e.g. "Line (A50 Stream Out)"

    /// AI proxy (OpenAI-compatible) — your Linux bridge.
    pub ai_base_url: String, // e.g. "http://192.168.0.142:18902/v1"
    pub ai_bearer: String,  // BRIDGE_SECRET
    pub ai_model: String,   // Live answers — fast, default claude-haiku-4-5
    pub prep_model: String, // Pre-meeting context structuring — smart, default claude-sonnet-4-5

    /// EXPERIMENTAL — when true, the system prompt is sent with Anthropic
    /// `cache_control: ephemeral` so a pass-through bridge can prompt-cache
    /// it (faster repeat/follow-up asks). Default OFF: some OpenAI-compat
    /// bridges reject the unknown field, so enable + test against YOUR
    /// bridge. `#[serde(default)]` keeps old config.json files loading.
    #[serde(default)]
    pub ai_prompt_cache: bool,

    /// Language tag (ISO 639-1) the assistant should ALWAYS respond in.
    /// Injected into the system prompt at runtime.
    pub response_language: String, // e.g. "ru"

    /// Groq Whisper STT.
    pub groq_api_key: String,
    pub stt_language: Option<String>, // None = auto-detect, "ru" = forced Russian
    /// Groq Whisper model: "whisper-large-v3" (most accurate, slower) vs
    /// "whisper-large-v3-turbo" (~3× faster, slightly less accurate).
    /// Default: large-v3 — accuracy beats latency for interview use.
    pub stt_model: String,

    /// Preferred monitor name for tile windows. None = first non-primary, fallback to primary.
    pub tile_monitor_name: Option<String>,

    /// Whitespace-separated trigger keywords for auto-tile spawn (case-insensitive).
    /// Example: "kubernetes etcd terraform postgres". Plus any "?" sentence.
    pub trigger_keywords: String,

    /// Enable auto-detect of questions/keywords in transcript → spawn tiles.
    pub auto_tiles_enabled: bool,

    /// When true, overlay + tile windows call set_content_protected(true) and
    /// become invisible to screen-share / capture APIs. OFF by default for
    /// easier debugging and use cases where stealth is not needed.
    pub stealth_enabled: bool,

    /// When true, on session stop the full mic transcript is sent to the
    /// prep_model (Sonnet by default) for a 3-point coaching debrief, which
    /// spawns as a Manual tile. Costs ~1 Sonnet call per session. Skipped
    /// when the session was shorter than 30s or had fewer than 5 mic lines.
    ///
    /// Default is OFF (opt-in via Settings). A privacy/cost-conscious tool
    /// shouldn't silently start spending money on Sonnet just because the
    /// user upgraded.
    #[serde(default = "default_post_meeting_debrief_enabled")]
    pub post_meeting_debrief_enabled: bool,

    /// v0.0.98: arbitrary CSS injected into the overlay window at
    /// mount time. Lets power users theme the overlay (color
    /// overrides, font tweaks, chip styling) without recompiling.
    /// Capped at 8 KB on the frontend so a typo doesn't blow up
    /// localStorage / IPC. Default empty.
    #[serde(default)]
    pub custom_css: String,

    /// v0.0.73: when true, `quit_app` exports the most recent session's
    /// JSONL journal to a Markdown file on the user's Desktop right
    /// before exiting. Filename: `suflyor-session-YYYY-MM-DD-HHmm.md`.
    /// Same rendering as the Replay viewer's "📥 Export markdown" button
    /// — only Q+A pairs + final summary, no raw transcript clutter.
    ///
    /// Default OFF (opt-in). Users who want every session captured
    /// without thinking enable it once and forget. Failure to write is
    /// logged but never blocks the quit (avoids "I want to leave but
    /// the app won't let me" UX nightmares).
    #[serde(default)]
    pub auto_export_on_quit: bool,

    /// Soft budget hint per session, in USD. When session cost crosses
    /// this number, a yellow "💰 over $X budget" chip appears in the
    /// overlay — but AI calls still go through. Blocking mid-interview
    /// would be terrible UX (you can't get help precisely when you need
    /// it). The rate-limit (15 auto-tiles/min) already prevents real
    /// runaway-spend scenarios.
    ///
    /// Set to 0 to disable the warning entirely.
    /// Default 1.00 USD ≈ 200 Haiku tile spawns. Counter resets on
    /// start_session.
    ///
    /// Live regression 2026-05-25: original v0.0.2 design was a HARD
    /// block, which user rightfully called "странное решение" — pivoted
    /// to soft warning in v0.0.5.
    #[serde(default = "default_max_session_cost_usd")]
    pub max_session_cost_usd: f64,

    /// When true, the auto-tile detector ignores transcript lines that
    /// came from the MICROPHONE (your own voice). Only system-audio lines
    /// (interviewer questions) can trigger an auto-tile. Live regression
    /// 2026-05-25: detector kept firing on the candidate's own statements
    /// ("Я работал с Kubernetes …") and spawned redundant explanation tiles.
    ///
    /// Default ON — interview use-case is "they ask, I answer; AI helps
    /// the answer." If you want both sides considered, turn this off.
    #[serde(default = "default_detector_skip_mic")]
    pub detector_skip_mic: bool,

    /// **AGGRESSIVE MODE** (v0.0.18). When true, `maybe_spawn_tile` skips
    /// the question/keyword detector entirely and treats EVERY transcript
    /// line as a trigger. Combined with `detector_skip_mic=false` this
    /// effectively spawns a tile for every audio chunk Whisper produces.
    ///
    /// Use cases:
    ///   - You're paying for AI and want maximum coverage regardless of
    ///     whether the line "sounds like a question"
    ///   - You're testing the pipeline end-to-end
    ///   - Whisper is dropping `?` and the candidate's monologue is what
    ///     you actually want suggestions on
    ///
    /// Trade-off: cost. With this on, expect 30-50 tiles per minute of
    /// continuous speech, each = one Haiku call. Soft cost cap chip still
    /// fires but doesn't block. Also bumps internal MAX_TILES_PER_MIN from
    /// 15 to 60 so the rate-limiter doesn't strangle aggressive mode.
    ///
    /// Default OFF — out of the box behaviour stays the same.
    #[serde(default)]
    pub auto_tile_every_line: bool,

    /// Hotkeys (cross-platform syntax, e.g. "F9", "CmdOrCtrl+Shift+A").
    pub hotkey_ask: String,
    pub hotkey_screenshot: String,
    pub hotkey_toggle_visibility: String,
    pub hotkey_pause_audio: String,

    /// How the manual-ask buttons (🎤 mic / 🔊 system) behave:
    /// - "click": single click → take last 5 lines, ask AI (current default)
    /// - "hold":  press-and-hold → records only what's said while held →
    ///   transcript window for that duration → ask AI
    ///
    /// Hold-mode shows a pulsing red indicator while recording.
    pub manual_ask_mode: String,

    /// UI language for Settings + Overlay + Tile chrome strings (NOT
    /// AI response language — that's `response_language` above). v0.0.42.
    /// Supported: "ru" (default, current primary), "en". Anything else
    /// falls back to "ru" at the t() lookup level.
    ///
    /// Stored in config.json. Loaded once per window mount; switching
    /// re-renders via React state. Tray menu remains Russian (Rust-side
    /// menu builder doesn't observe this field — separate concern).
    #[serde(default = "default_ui_language")]
    pub ui_language: String,

    /// Tile body font size in px. Default 12. Reasonable range 11-18.
    /// Stored here (not localStorage) because tile windows can't read
    /// localStorage from the overlay window — has to be passed via
    /// URL param (`&fs=14`) at spawn time. v0.0.55.
    #[serde(default = "default_tile_font_size")]
    pub tile_font_size: u32,

    /// Pre-written answer snippets. Each snippet has a short trigger key
    /// (e.g. "k8s", "pg") that the user can invoke via the palette to
    /// instantly spawn a tile with the body text — zero AI latency,
    /// zero cost. Great for the 5-6 "give me the template" questions
    /// that come up every interview (incident-response framework, SLI
    /// design, postgres tuning checklist, etc.).
    pub snippets: Vec<Snippet>,

    /// Phase E6 v20 — tile body opacity (0.5..1.0). Lets the user see
    /// THROUGH tiles to the meeting window underneath. Default 1.0 =
    /// opaque (current behaviour). Cherry-picked from the design
    /// bundle 2 `body-opacity` prop; only this one design change was
    /// adopted — see cycle 26 chat thread for risk-analysis rationale.
    #[serde(default = "default_tile_body_opacity")]
    pub tile_body_opacity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Snippet {
    /// Short trigger key, case-insensitive (e.g. "k8s", "pg-tune").
    pub key: String,
    /// Human-readable title shown as the tile's question text.
    pub title: String,
    /// Body — full markdown rendered in the tile.
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextProfile {
    pub name: String,
    pub context: String,
}

impl Config {
    pub fn defaults() -> Self {
        Self {
            meeting_context: String::new(),
            context_profiles: vec![],
            active_profile: None,
            mic_device: None,
            system_audio_device: None,
            ai_base_url: "http://192.168.0.142:18902/v1".into(),
            ai_bearer: String::new(),
            ai_model: "claude-haiku-4-5".into(),
            prep_model: "claude-sonnet-4-6".into(),
            ai_prompt_cache: false,
            response_language: "ru".into(),
            groq_api_key: String::new(),
            stt_language: Some("ru".into()),
            stt_model: "whisper-large-v3".into(),
            tile_monitor_name: None,
            stealth_enabled: false, // OFF by default — easier to debug & not every use case needs stealth
            trigger_keywords: default_trigger_keywords(),
            auto_tiles_enabled: true,
            hotkey_ask: "F9".into(),
            hotkey_screenshot: "F10".into(),
            hotkey_toggle_visibility: "F11".into(),
            hotkey_pause_audio: "F12".into(),
            manual_ask_mode: "hold".into(), // push-to-talk by default
            ui_language: default_ui_language(),
            tile_font_size: default_tile_font_size(),
            snippets: default_snippets(),
            post_meeting_debrief_enabled: default_post_meeting_debrief_enabled(),
            custom_css: String::new(),
            auto_export_on_quit: false,
            max_session_cost_usd: default_max_session_cost_usd(),
            detector_skip_mic: default_detector_skip_mic(),
            auto_tile_every_line: false,
            tile_body_opacity: default_tile_body_opacity(),
        }
    }
}

fn default_tile_body_opacity() -> f32 {
    1.0
}

fn default_post_meeting_debrief_enabled() -> bool {
    false // opt-in — surprise Sonnet calls are bad UX
}

fn default_max_session_cost_usd() -> f64 {
    // v0.0.28: flipped 1.00 → 0.0 (chip disabled by default). Pet-project
    // user explicitly opted out of cost guard rails — said «по костам не
    // важно, у меня безлимитные деньги». Old installs keep their existing
    // value (serde reads from file); the chip only stops appearing for
    // fresh installs OR users who explicitly set 0. The cost-cap field
    // remains in Settings for users who DO want it.
    //
    // The hard-block path was already removed in v0.0.5 (soft warning
    // only), so this is purely a UX guilt-trip removal, not a behaviour
    // change for live AI calls.
    0.0
}

fn default_detector_skip_mic() -> bool {
    true // candidate's own voice shouldn't trigger explanation tiles
}

fn default_ui_language() -> String {
    // v0.0.42: default RU because that's the current primary language
    // (user is Russian-speaking; original Settings copy is Russian).
    // EN is opt-in via Settings → Interface → Язык интерфейса.
    "ru".into()
}

fn default_tile_font_size() -> u32 {
    // v0.0.55: default 12 matches the historic `--fs-12` CSS var that
    // .tile-body.markdown had previously hardcoded. Range 11-18 keeps
    // tiles readable without breaking grid math.
    12
}

/// Massive default trigger-keyword pool — 250+ DevOps/SRE/Cloud/Linux
/// terms across every common interview domain. Detector fires on any
/// whole-word match. User-configurable in Settings.
///
/// Note: this string also feeds Whisper's bias prompt (alongside
/// CANONICAL_TECH_VOCAB) — heavy users on the 800-char prompt budget
/// may want to trim. The detector ignores prompt budget — match against
/// the full list always.
///
/// Curated 2026-05-25 by domain (line-grouped for readability).
fn default_trigger_keywords() -> String {
    "\
        kubernetes k8s k3s etcd helm kustomize argocd flux istio linkerd cilium calico \
        kubectl kubeadm kubelet ingress configmap daemonset statefulset deployment \
        \
        docker containerd podman runc crio buildkit dockerfile compose multistage \
        registry distroless oci namespace cgroup \
        \
        linux bash zsh systemd journalctl strace ltrace lsof tcpdump iptables \
        nftables ufw firewalld selinux apparmor iotop htop dstat sar perf flamegraph \
        \
        postgres pgbouncer mysql mariadb mongo mongodb redis memcached rabbitmq \
        kafka nats activemq pulsar cassandra clickhouse cockroachdb elasticsearch \
        influxdb timescaledb prometheus opensearch \
        \
        grafana loki tempo jaeger zipkin opentelemetry alertmanager fluentd \
        fluentbit vector datadog newrelic splunk pagerduty observability tracing \
        sli slo sla errorbudget runbook postmortem chaos \
        \
        terraform ansible puppet chef saltstack pulumi crossplane vagrant packer \
        consul vault nomad opa rego sentinel \
        \
        jenkins gitlab github bitbucket teamcity bamboo circleci travis drone \
        argo flux helm tekton skaffold spinnaker gitops cicd pipeline \
        \
        aws gcp azure ec2 s3 rds eks gke aks lambda dynamodb sqs sns kinesis \
        cloudwatch cloudfront route53 elb alb nlb vpc subnet iam sts kms \
        bigquery pubsub cloudsql functions appengine cloudrun \
        eventhub servicebus cosmosdb storage blob queues vmss aks app-service \
        \
        nginx haproxy envoy traefik caddy apache varnish istio linkerd \
        gateway service-mesh sidecar canary blue-green rolling \
        \
        tcp udp http https grpc rest graphql websocket dns bgp ospf vpn vxlan \
        mpls nat dhcp dhcpv6 ipv4 ipv6 mtu mss tls ssl mtls handshake \
        certificate ca pki acme letsencrypt \
        \
        load balancer latency throughput jitter packet-loss bandwidth pps \
        connection pool keepalive timeout retry backoff circuit-breaker \
        ratelimit deadlock contention concurrency parallelism \
        \
        cpu memory disk ram nvme ssd hdd iops queue swap ballooning hugepages \
        oom segfault corefile coredump panic kernel module driver \
        \
        ci cd cicd devops sre devsecops gitops trunk-based mvc microservices \
        monolith serverless event-driven cqrs saga eventsourcing \
        \
        python golang rust java kotlin scala swift typescript javascript \
        nodejs npm pnpm yarn cargo maven gradle webpack vite esbuild \
        \
        oauth oidc jwt saml sso mtls rbac abac ldap kerberos zerotrust \
        encryption hashing bcrypt argon2 hmac signing certificate-pinning \
        secrets rotation \
        \
        cache invalidation cdn write-through write-back write-around eviction \
        lru lfu ttl stampede coherence consistency partition replication sharding \
        \
        scaling autoscaling vertical horizontal hpa vpa keda spot-instance \
        capacity provisioning forecasting throughput-test \
        \
        backup snapshot restore disaster-recovery rto rpo failover failback \
        active-passive active-active region availability-zone\
    "
    .into()
}

/// Massive default snippet library — 50+ pre-written templates covering
/// every common SRE / DevOps interview topic. Each is dense (~500-1000
/// chars), Russian-language, markdown-formatted, ready to spawn as a tile
/// via `/key` palette. Zero AI call, zero cost, ~50ms latency.
///
/// Curated 2026-05-25 from real interview question pool + production
/// runbook commons. Edit `%APPDATA%\overlay-mvp\config.json` array
/// `snippets` to customise per-user.
fn default_snippets() -> Vec<Snippet> {
    vec![
                Snippet {
                    key: "k8s".into(),
                    title: "Kubernetes troubleshoot — 5-step framework".into(),
                    body: "**Шаги диагностики (по убыванию частоты):**\n\n\
                           1. `kubectl get pods -A | grep -v Running` — что не Running?\n\
                           2. `kubectl describe pod X` — Events внизу: ImagePullBackOff / CrashLoopBackOff / OOMKilled / Pending?\n\
                           3. `kubectl logs X --previous` — последний exit, особенно для CrashLoop\n\
                           4. `kubectl get events --sort-by=.lastTimestamp` — cluster-wide контекст\n\
                           5. **Node-level:** `kubectl top node`, `df -h`, `dmesg` — диск/память/OOM?\n\n\
                           **Корень причин в нашей практике (топ 5):**\n\
                           - readiness/liveness probe слишком агрессивная → kill loop\n\
                           - ImagePullSecret истёк / private registry\n\
                           - Resource requests > capacity → Pending forever\n\
                           - PVC stuck (PV из другой AZ)\n\
                           - DNS внутри cluster: `nslookup kubernetes.default` в Pod'е".into(),
                },
                Snippet {
                    key: "pg".into(),
                    title: "PostgreSQL slow query — что проверять".into(),
                    body: "**Чеклист (порядок имеет значение):**\n\n\
                           1. **`EXPLAIN (ANALYZE, BUFFERS)`** — seq scan на большой таблице? индекса нет?\n\
                           2. `pg_stat_statements` — кто топ-10 по total_time?\n\
                           3. **Bloat:** `pg_stat_user_tables.n_dead_tup` — autovacuum успевает?\n\
                           4. **Locks:** `pg_stat_activity` где `wait_event_type='Lock'`\n\
                           5. **Config sanity:** `shared_buffers` (~25% RAM), `effective_cache_size` (~75%), `work_mem` × max_connections не превышает свободную RAM\n\n\
                           **Частые подставы:**\n\
                           - `SET random_page_cost = 1.1` для NVMe (default 4.0 — ложь на современных дисках)\n\
                           - JIT включён на маленьких запросах → ↑latency. `jit=off` для OLTP\n\
                           - Connection pooler отсутствует → 1000+ idle процессов жрут RAM. **PgBouncer transaction mode**".into(),
                },
                Snippet {
                    key: "incident".into(),
                    title: "Incident response — первые 5 минут".into(),
                    body: "**Order of operations:**\n\n\
                           1. **Признать:** «вижу алерт X, начинаю расследование». Без этого все ждут.\n\
                           2. **Stop the bleed (не root cause!):** rollback / failover / scale up. Лечим симптом сначала.\n\
                           3. **Open war room** + один **incident commander** (только координирует, не дебажит)\n\
                           4. **Timeline в realtime:** `T+0 alert, T+2 rollback started, T+5 mitigated…`\n\
                           5. **Communication on schedule:** статус каждые 15 мин даже если «still investigating»\n\n\
                           **NEVER** в первые 5 минут:\n\
                           - искать виноватого\n\
                           - чинить config in-place без бэкапа\n\
                           - молча копаться 30 минут «я почти нашёл»\n\n\
                           **Post-mortem:** blameless, 5 whys, action items с owner+due date".into(),
                },
                Snippet {
                    key: "sli".into(),
                    title: "SLI/SLO design — что измерять, что НЕ измерять".into(),
                    body: "**Хорошие SLI** (user-visible):\n\
                           - **Availability:** % успешных HTTP 200-399 за окно\n\
                           - **Latency:** p99 ≤ X ms для critical path\n\
                           - **Throughput:** requests/sec для batch жоб\n\
                           - **Correctness:** % правильных ответов (для ML/search)\n\n\
                           **Плохие SLI** (proxy-метрики, не user-pain):\n\
                           - CPU usage, RAM usage — никого не волнует пока система работает\n\
                           - Pod restarts — может быть «правильно» (rolling deploy)\n\n\
                           **Error budget:** SLO 99.9% = 43min downtime/month. Если бюджет сгорел — **stop feature releases, focus on reliability**. Не «прибавим строгости» — продакшн уже сгорел.\n\n\
                           **SLO ≠ SLA.** SLA = договорное обещание клиенту (с штрафами). SLO = внутренний таргет, обычно строже SLA.".into(),
                },
                // ── Kubernetes deep cuts ──────────────────────────────
                Snippet { key: "k8s-net".into(), title: "K8s networking — Service / Ingress / CNI".into(), body:
                    "**Service types (от меньшего scope):**\n\
                     - **ClusterIP** — внутри cluster, default\n\
                     - **NodePort** — открывает 30000-32767 на каждом node (dev/staging)\n\
                     - **LoadBalancer** — облачный LB (AWS NLB, GCP TCP LB)\n\
                     - **ExternalName** — CNAME alias, без proxy\n\n\
                     **Ingress vs Service:** Service = L4 (TCP/UDP), Ingress = L7 (HTTP host/path routing, TLS termination). Без Ingress controller (nginx-ingress, traefik, contour) ресурс Ingress ничего не делает.\n\n\
                     **CNI plugins (для interview):** Calico (BGP, NetworkPolicy), Cilium (eBPF, observability), Flannel (простой, VXLAN), Weave (mesh, encrypted). Выбор зависит от: NetworkPolicy support, performance, encryption needs.\n\n\
                     **Debug:** `kubectl exec -it pod -- nslookup svc-name`, `iptables -L -n -t nat | grep PORT`, `cilium monitor` для eBPF cluster.".into() },
                Snippet { key: "k8s-rbac".into(), title: "K8s RBAC — Roles, Bindings, SA".into(), body:
                    "**4 главных объекта:**\n\
                     - **Role / ClusterRole** — *что можно* (verbs: get/list/watch/create/update/patch/delete)\n\
                     - **RoleBinding / ClusterRoleBinding** — *кому* (subjects: User, Group, ServiceAccount)\n\
                     - **ServiceAccount** — идентичность для Pod'а (по умолчанию `default` в namespace)\n\
                     - **API Group** в Role — `\"\"` для core (pods, services), `apps` для deployments\n\n\
                     **Принципы:**\n\
                     - **Least privilege:** Role > ClusterRole когда хватает namespace\n\
                     - Default SA НЕ давать прав — создавать отдельный `app-sa` для каждого workload\n\
                     - `automountServiceAccountToken: false` если Pod не использует API\n\n\
                     **Debug:** `kubectl auth can-i create pods --as=system:serviceaccount:default:app-sa`".into() },
                Snippet { key: "k8s-storage".into(), title: "K8s storage — PV / PVC / StorageClass".into(), body:
                    "**Chain:** App → **PVC** (запрос storage) → **PV** (фактический volume) → **StorageClass** (provisioner).\n\n\
                     **Access modes:**\n\
                     - `ReadWriteOnce (RWO)` — один node (CSI block, EBS, GP3)\n\
                     - `ReadWriteMany (RWX)` — несколько nodes (NFS, EFS, CephFS)\n\
                     - `ReadOnlyMany (ROX)` — read-only несколько nodes\n\n\
                     **StorageClass важные параметры:**\n\
                     - `reclaimPolicy: Retain` — НЕ удалять PV при удалении PVC (важные данные)\n\
                     - `volumeBindingMode: WaitForFirstConsumer` — создавать PV в той же zone что Pod\n\n\
                     **Частые pain points:**\n\
                     - PVC `Pending` — нет StorageClass или provisioner не запущен\n\
                     - Pod `Pending` — PV в другой AZ от node\n\
                     - StatefulSet с RWO + node failure → manual recovery нужна".into() },
                Snippet { key: "k8s-autoscale".into(), title: "K8s autoscaling — HPA / VPA / Cluster Autoscaler".into(), body:
                    "**Три уровня:**\n\
                     - **HPA** (Horizontal Pod Autoscaler) — увеличивает количество Pod'ов по CPU/memory/custom metric\n\
                     - **VPA** (Vertical Pod Autoscaler) — меняет requests/limits существующих Pod'ов\n\
                     - **CA** (Cluster Autoscaler) — добавляет/убирает nodes когда Pod'ы Pending\n\n\
                     **HPA подводные камни:**\n\
                     - Metric server должен быть установлен (`kubectl top pod` работает?)\n\
                     - Stabilization window: scale-up быстрый, scale-down медленный (5 мин default)\n\
                     - Custom metrics — нужен Prometheus Adapter\n\n\
                     **VPA + HPA конфликт:** один меняет requests, другой решает по % использования. Использовать вместе только с external metric, не CPU.\n\n\
                     **KEDA** — event-driven autoscaling (Kafka lag, SQS queue depth, etc.). Альтернатива HPA когда CPU не отражает нагрузку.".into() },
                Snippet { key: "k8s-secrets".into(), title: "K8s secrets — что хранить, как защищать".into(), body:
                    "**Default Secret = base64**, не шифрование. На диске etcd лежит как plaintext.\n\n\
                     **Защита:**\n\
                     - **Encryption at rest:** `--encryption-provider-config` в API server (AES-CBC / KMS provider)\n\
                     - **External secret store:** Vault, AWS Secrets Manager, GCP Secret Manager — через External Secrets Operator (ESO) или Vault Agent Injector\n\
                     - **Sealed Secrets** (Bitnami) — шифруем secret через public key, безопасно коммитим в Git\n\
                     - **SOPS + age** — encrypted YAML в Git, GitOps friendly\n\n\
                     **Принципы:**\n\
                     - НЕ хранить secrets в env vars видимых через `kubectl describe pod`\n\
                     - Mount как files (volumeMount), `defaultMode: 0400`\n\
                     - Rotation: short TTL + автоматическая инъекция (Vault dynamic secrets)\n\
                     - RBAC: `kubectl auth can-i get secrets -n prod` от service account".into() },
                // ── Linux troubleshooting ─────────────────────────────
                Snippet { key: "linux-oom".into(), title: "Linux OOM killer — кто и почему".into(), body:
                    "**Симптомы:** процесс пропал без stacktrace, в `dmesg` строка `Out of memory: Killed process X (name)`.\n\n\
                     **Расследование:**\n\
                     1. `dmesg -T | grep -i 'killed process'` — кто, когда, score\n\
                     2. `cat /proc/<pid>/oom_score` — оценка перед kill (выше = первый кандидат)\n\
                     3. `/var/log/messages` или `journalctl -k --since '1 hour ago'`\n\
                     4. **Контекст:** общая память до момента — `sar -r 1`, `free -h`\n\n\
                     **Профилактика:**\n\
                     - **cgroup memory limits** — контейнер OOM'ится первым, не всё на хосте\n\
                     - `vm.overcommit_memory=2` + `overcommit_ratio` — строгий contract вместо optimistic\n\
                     - `oom_score_adj` критичным процессам (database, prometheus): `-1000` = неубиваемый\n\
                     - Включить **swap** (даже на K8s nodes — `--fail-swap-on=false`)\n\n\
                     **K8s context:** OOMKilled в `kubectl describe pod` — увеличить `resources.limits.memory`.".into() },
                Snippet { key: "linux-disk".into(), title: "Linux диск переполнен — как разобраться".into(), body:
                    "**Симптомы:** `Write failed: No space left on device`, app падает.\n\n\
                     **Что проверять:**\n\
                     1. **`df -h`** — какой раздел? (часто `/var/log` или `/tmp`)\n\
                     2. **`df -i`** — inodes! Маленькие файлы могут забить inodes, не block usage\n\
                     3. **`du -hx --max-depth=1 /var | sort -h`** — найти жирный subdir\n\
                     4. **`lsof | grep deleted | sort -k7 -h`** — открытые удалённые файлы (rotated logs, что app держит) — занимают место до restart\n\
                     5. **`ncdu /`** — интерактивный TUI, быстрее `du`\n\n\
                     **Частые причины:**\n\
                     - Logs без logrotate (rotated) → growing forever\n\
                     - `journalctl --vacuum-size=500M` — systemd journal раздулся\n\
                     - `docker system prune -a` — image cache, build cache\n\
                     - Core dumps в `/var/lib/systemd/coredump/`\n\
                     - **`lsof +L1`** — файлы с link-count 0 (deleted, still held)".into() },
                Snippet { key: "linux-net".into(), title: "Linux network debug — что-то не отвечает".into(), body:
                    "**Слой за слоем, снизу вверх:**\n\n\
                     **L1-L2 (link):** `ip link show` — UP/DOWN? `ethtool eth0` — скорость/duplex.\n\
                     **L3 (IP):** `ip addr show`, `ip route get 8.8.8.8` — какой интерфейс/gateway.\n\
                     **L3 connectivity:** `ping -c 4 <gateway>` → `ping 8.8.8.8` → `ping google.com`. Где сломалось?\n\
                     **DNS:** `dig +short example.com`, `getent hosts example.com` (учитывает /etc/hosts).\n\
                     **L4 connectivity:** `nc -vz host 443`, `curl -v https://host`, `traceroute -n -T -p 443 host`.\n\n\
                     **Полезные tools:**\n\
                     - `ss -tnp` — кто слушает (быстрее netstat)\n\
                     - `ss -tn state established` — активные коннекшены\n\
                     - `tcpdump -i any -nn host X.X.X.X` — что реально летит\n\
                     - `iptables -L -n -v`, `nft list ruleset` — фаервол блокирует?\n\
                     - `mtr <host>` — combination traceroute + ping, видит intermittent loss".into() },
                Snippet { key: "linux-perf".into(), title: "Linux performance — USE method (Brendan Gregg)".into(), body:
                    "**USE = Utilization · Saturation · Errors** для каждого ресурса:\n\n\
                     **CPU:**\n\
                     - Utilization: `top`, `mpstat -P ALL 1`\n\
                     - Saturation: load average / cores > 1.0\n\
                     - Errors: `dmesg | grep -i 'mce\\|cpu'`\n\n\
                     **Memory:**\n\
                     - U: `free -h`, `vmstat 1`\n\
                     - S: swap I/O (`si/so` в vmstat) — не нулевые? oom-kill recent?\n\
                     - E: ECC errors (`edac-util`)\n\n\
                     **Disk:**\n\
                     - U: `iostat -xz 1` (%util)\n\
                     - S: avgqu-sz (queue), await/svctm — wait time\n\
                     - E: `smartctl -a /dev/sda`, `dmesg | grep -i error`\n\n\
                     **Network:**\n\
                     - U: `sar -n DEV 1`, `iftop`\n\
                     - S: `ss -s` (overflow, retransmits), `nstat | grep -i drop`\n\
                     - E: `ip -s link` (errors/dropped)\n\n\
                     **Профайлеры:** `perf top`, `perf record/report`, `bcc-tools` (eBPF), `flamegraph.pl`.".into() },
                Snippet { key: "linux-systemd".into(), title: "systemd — основные команды + unit files".into(), body:
                    "**Status / control:**\n\
                     - `systemctl status <unit>` — текущее состояние\n\
                     - `systemctl start/stop/restart/reload <unit>`\n\
                     - `systemctl enable/disable <unit>` — boot persistence\n\
                     - `systemctl list-units --failed` — что упало\n\n\
                     **Journals:**\n\
                     - `journalctl -u <unit> -f` — tail\n\
                     - `journalctl -u <unit> --since '1 hour ago'`\n\
                     - `journalctl -p err -b` — errors с последнего boot\n\n\
                     **Unit file (`/etc/systemd/system/myapp.service`):**\n\
                     ```ini\n\
                     [Unit]\n\
                     Description=My App\n\
                     After=network-online.target\n\
                     Wants=network-online.target\n\n\
                     [Service]\n\
                     Type=notify\n\
                     ExecStart=/usr/bin/myapp\n\
                     Restart=on-failure\n\
                     RestartSec=5s\n\
                     User=myapp\n\
                     MemoryMax=2G\n\
                     CPUQuota=200%\n\n\
                     [Install]\n\
                     WantedBy=multi-user.target\n\
                     ```\n\n\
                     После правок: `systemctl daemon-reload && systemctl restart myapp`.".into() },
                // ── Networking deep cuts ──────────────────────────────
                Snippet { key: "tcp".into(), title: "TCP states + 3-way handshake + проблемы".into(), body:
                    "**3-way handshake:** SYN → SYN+ACK → ACK. После — `ESTABLISHED`.\n\
                     **Close:** FIN → ACK → FIN → ACK. Между FIN+ACK и финальным ACK — `TIME_WAIT` (~60s).\n\n\
                     **Состояния которые видишь в `ss`:**\n\
                     - `LISTEN` — server слушает\n\
                     - `ESTAB` — рабочее соединение\n\
                     - `TIME_WAIT` — много = частые короткие коннекты, нужен keep-alive\n\
                     - `CLOSE_WAIT` — твой код не закрыл socket после remote FIN. **Bug в app**\n\
                     - `SYN_SENT` зависший — firewall дропает или пакеты теряются\n\n\
                     **TCP tuning для high-throughput:**\n\
                     - `net.core.somaxconn=65535` — backlog accept queue\n\
                     - `net.ipv4.tcp_max_syn_backlog=65535`\n\
                     - `net.ipv4.tcp_fin_timeout=15` — короче TIME_WAIT (если backend behind LB)\n\
                     - `net.ipv4.tcp_tw_reuse=1` — переиспользовать TIME_WAIT sockets\n\
                     - `net.core.netdev_max_backlog=5000` — pre-routing queue\n\n\
                     **MTU issues:** `tracepath host`, `ping -M do -s 1472 host` — если фрагментация ломает MSS clamping, проблема в туннеле.".into() },
                Snippet { key: "dns".into(), title: "DNS — как работает + диагностика".into(), body:
                    "**Иерархия резолвинга (от хоста):**\n\
                     1. `/etc/hosts` (статический)\n\
                     2. **NSS** (`/etc/nsswitch.conf` — `hosts: files dns`)\n\
                     3. `systemd-resolved` (если активен) — кеширует, читает `/etc/systemd/resolved.conf`\n\
                     4. `/etc/resolv.conf` — recursive resolvers (8.8.8.8, 1.1.1.1)\n\
                     5. Recursive resolver обходит: root → TLD (.com) → authoritative для example.com\n\n\
                     **Tools (используй в этом порядке):**\n\
                     - `getent hosts example.com` — учитывает /etc/hosts + nsswitch\n\
                     - `dig +short example.com` — pure DNS query\n\
                     - `dig +trace example.com` — полный обход иерархии\n\
                     - `dig @8.8.8.8 example.com` — конкретный resolver\n\
                     - `nslookup -debug` — старый, иногда полезен для verbose response\n\n\
                     **Частые проблемы:**\n\
                     - TTL = 0 → каждый запрос пересчитывается → latency\n\
                     - search-domain в resolv.conf → лишние NXDOMAIN запросы\n\
                     - Coredns в K8s: `kubectl exec -it pod -- nslookup kubernetes.default`\n\
                     - DNS-over-HTTPS (DoH) — Cloudflare/Quad9 для приватности".into() },
                Snippet { key: "tls".into(), title: "TLS handshake + сертификаты + типичные ошибки".into(), body:
                    "**TLS 1.3 handshake (упрощённо):**\n\
                     1. Client → Server: `ClientHello` (поддерживаемые ciphers, SNI hostname, key share)\n\
                     2. Server → Client: `ServerHello` + cert + key share. Уже шифровано после этого\n\
                     3. Client verify cert → derive shared key → `Finished`. Готово, 1-RTT.\n\n\
                     **TLS 1.2 = 2 RTT** (старый, не используй для новых сервисов).\n\n\
                     **Cert chain:** leaf → intermediate(s) → root CA. **Сервер ДОЛЖЕН отдавать leaf + intermediates** (не root — он у клиента).\n\n\
                     **Debug:**\n\
                     - `openssl s_client -connect host:443 -servername host` — handshake debug, видит весь chain\n\
                     - `curl -vI https://host` — verbose с TLS info\n\
                     - `ssllabs.com/ssltest` — внешняя проверка\n\n\
                     **Типичные ошибки:**\n\
                     - `unable to verify the first certificate` — не отдан intermediate\n\
                     - `Hostname mismatch` — cert на `www.x.com`, ходишь на `x.com` (нужен SAN)\n\
                     - `certificate has expired` — поставь `cert-manager` + ACME (Let's Encrypt)\n\
                     - `wrong version number` — кто-то говорит HTTP вместо HTTPS на port 443".into() },
                Snippet { key: "lb".into(), title: "Load balancers — типы, алгоритмы, sticky sessions".into(), body:
                    "**L4 vs L7:**\n\
                     - **L4** (TCP/UDP) — AWS NLB, HAProxy mode TCP. Быстро, не знает HTTP. Можно балансить gRPC, MQTT, Postgres.\n\
                     - **L7** (HTTP) — AWS ALB, nginx, Envoy. Видит headers/paths → routing rules, TLS termination, rewrite. Дороже CPU.\n\n\
                     **Алгоритмы:**\n\
                     - **Round-robin** — простой, не учитывает нагрузку\n\
                     - **Least connections** — лучше для long-lived (websocket, БД pool)\n\
                     - **IP hash / consistent hash** — кеш friendly (один user → один backend), но плохой spread\n\
                     - **Random with two choices (P2C)** — на удивление хорошо работает\n\
                     - **Weighted** — backend с разным CPU\n\n\
                     **Sticky sessions:**\n\
                     - Cookie-based (L7): LB вставляет `AWSALB=xxx`\n\
                     - Source-IP (L4): hash (ip, port) → backend. Ломается за NAT\n\
                     - **Избегай если можно** — stateless app + Redis session > sticky\n\n\
                     **Health checks:** `/health` endpoint, interval 5-10s, threshold 2-3 fails. **Не путать с liveness/readiness в K8s.**".into() },
                Snippet { key: "http".into(), title: "HTTP коды — что значат + когда используются".into(), body:
                    "**2xx success:**\n\
                     - `200 OK` — стандартный успех\n\
                     - `201 Created` — POST создал ресурс (Location header указывает на новый)\n\
                     - `204 No Content` — успех но тела нет (DELETE, PUT без response)\n\n\
                     **3xx redirect:**\n\
                     - `301` — permanent (SEO friendly, кеш forever)\n\
                     - `302` — temporary (default Express/Flask `redirect`)\n\
                     - `304 Not Modified` — ETag/If-Modified-Since совпали\n\n\
                     **4xx client error:**\n\
                     - `400` — невалидный request (badly formed JSON)\n\
                     - `401` — нет credentials (отдай `WWW-Authenticate`)\n\
                     - `403` — credentials есть, но прав нет\n\
                     - `404` — ресурс не существует\n\
                     - `409` — конфликт (optimistic lock, version mismatch)\n\
                     - `422 Unprocessable Entity` — JSON валиден но семантически ломан\n\
                     - `429 Too Many Requests` — rate limit (отдай `Retry-After`)\n\n\
                     **5xx server error:**\n\
                     - `500` — что-то сломалось внутри (не пиши stack trace в body!)\n\
                     - `502 Bad Gateway` — proxy не достучался до upstream\n\
                     - `503 Service Unavailable` — temporary, scheduled maintenance, отдай `Retry-After`\n\
                     - `504 Gateway Timeout` — upstream timeout".into() },
                // ── Databases ─────────────────────────────────────────
                Snippet { key: "pg-replica".into(), title: "PostgreSQL replication — streaming, logical, варианты".into(), body:
                    "**Streaming replication (binary, физический WAL):**\n\
                     - Setup: `pg_basebackup -h primary -U replicator -D /var/lib/postgresql -R`\n\
                     - Replica = read-only (default `hot_standby = on`)\n\
                     - Полная копия cluster — нельзя реплицировать одну DB\n\
                     - Async (default) или sync (`synchronous_standby_names`)\n\n\
                     **Logical replication (per-table, начиная с PG10):**\n\
                     - Publisher: `CREATE PUBLICATION pub FOR TABLE users, orders;`\n\
                     - Subscriber: `CREATE SUBSCRIPTION sub CONNECTION '...' PUBLICATION pub;`\n\
                     - Можно cross-version (10 → 16), можно частично, можно writeable\n\
                     - НЕ реплицирует DDL (схемы должны совпадать manually)\n\n\
                     **HA paterns:**\n\
                     - **Patroni** — leader election через etcd/consul/zk + auto-failover\n\
                     - **repmgr** — старый, manual switchover\n\
                     - **PgBouncer / Pgpool-II** — pooling + read/write split\n\n\
                     **Подводные камни:**\n\
                     - WAL bloat если replica отстаёт → `max_slot_wal_keep_size` (PG13+) спасает\n\
                     - Split-brain при failover без fencing — terminate old primary жёстко".into() },
                Snippet { key: "mysql".into(), title: "MySQL replication + InnoDB ключевые особенности".into(), body:
                    "**Replication типы:**\n\
                     - **Statement-based** — replicate SQL текст. Проблемы с non-deterministic (`NOW()`, `RAND()`)\n\
                     - **Row-based** (default 5.7+) — реплицируем сами row changes\n\
                     - **Mixed** — Statement когда безопасно, иначе Row\n\
                     - **GTID** (Global Transaction ID) — упрощает failover, обязателен для group replication\n\n\
                     **InnoDB важное:**\n\
                     - **Buffer pool** — главный cache. `innodb_buffer_pool_size = 70-80% RAM`\n\
                     - **Redo log** (`ib_logfile0/1`) — write-ahead, recovery после crash\n\
                     - **Undo log** — MVCC read consistency, rollback\n\
                     - **Clustered index** — таблица физически отсортирована по PK. Без PK MySQL создаст hidden\n\
                     - **Secondary index** содержит PK, не row pointer → wide PK = wide indexes\n\n\
                     **Tuning:**\n\
                     - `innodb_flush_log_at_trx_commit=1` (durability) vs `=2` (perf, риск 1s loss)\n\
                     - `innodb_io_capacity=2000` для SSD (default 200)\n\
                     - `sync_binlog=1` для prod (производительность ↓, durability ↑)".into() },
                Snippet { key: "redis".into(), title: "Redis — persistence, cluster, типичные паттерны".into(), body:
                    "**Persistence:**\n\
                     - **RDB** — snapshot периодически (`save 300 10`). Fast restart, может потерять последние секунды\n\
                     - **AOF** — append-only log каждой write op. `appendfsync everysec` (compromise) или `always` (slow)\n\
                     - **Both** — Redis читает AOF при старте. Recommended для prod\n\n\
                     **HA / scaling:**\n\
                     - **Sentinel** — мониторинг master/replicas + auto-failover (HA только)\n\
                     - **Cluster** — 16384 slots, sharded (data разбита по nodes). Min 3 masters + 3 replicas\n\
                     - **Cluster ограничения:** multi-key ops только если все keys в одном slot (`{user:1}:foo`, `{user:1}:bar`)\n\n\
                     **Паттерны:**\n\
                     - **Cache-aside:** app сам читает/пишет cache. Простой, fault-tolerant.\n\
                     - **Rate limit:** `INCR + EXPIRE` или token bucket через Lua\n\
                     - **Distributed lock:** Redlock алгоритм (controversial; `SET key val NX PX 30000` достаточно для большинства)\n\
                     - **Pub/Sub** — fire-and-forget, без persistence. Для гарантий → Streams\n\n\
                     **Anti-patterns:** `KEYS *` в prod (заблокирует), большие values (>10MB), expensive Lua scripts.".into() },
                Snippet { key: "mongo".into(), title: "MongoDB — replica set, sharding, indexing".into(), body:
                    "**Replica set:**\n\
                     - Min 3 nodes (primary + 2 secondary) — для election quorum\n\
                     - Primary только пишет, secondary читают (если `readPreference != primary`)\n\
                     - Election при недоступности primary, ~10s timeout\n\
                     - **Oplog** — capped collection, source-of-truth для replication\n\n\
                     **Sharding (для huge datasets):**\n\
                     - **Shard key** — главное решение. Низкая cardinality = hotspot. Не меняется после установки.\n\
                     - **Hashed shard key** — равномерный spread, но range queries разбиваются на все shards\n\
                     - **Compound shard key** — лучше, но всё равно immutable\n\
                     - Components: `mongos` (router), `config servers` (3-node replica set с metadata), `shards`\n\n\
                     **Indexes:**\n\
                     - Compound: ESR-правило (Equality, Sort, Range) — порядок полей\n\
                     - **Covered query** — все нужные fields есть в index → не trip to documents\n\
                     - **Partial index** — `{partialFilterExpression: {active: true}}` экономит место\n\
                     - `db.collection.explain('executionStats').find(...)` — есть IXSCAN или COLLSCAN?".into() },
                Snippet { key: "ch".into(), title: "ClickHouse — для interview SRE/data".into(), body:
                    "**Что это:** column-oriented OLAP DB. Optimized для агрегаций по миллиардам строк. **НЕ заменяет** OLTP (Postgres/MySQL).\n\n\
                     **Ключевые особенности:**\n\
                     - **Columnar storage** — читает только нужные columns, компрессия высокая (LZ4/ZSTD)\n\
                     - **MergeTree** family — основной engine. Данные иммутабельны, periodic background merge\n\
                     - **Sharding + replication** через ZooKeeper / ClickHouse Keeper (встроенный, PG-like)\n\
                     - **Materialized views** — pre-aggregations, обновляются по INSERT\n\n\
                     **Подходит для:** logs (Loki alternative), metrics (Prometheus long-term), analytics, observability backend (Datadog using CH internally).\n\n\
                     **НЕ подходит для:** транзакций, UPDATE-heavy workloads, primary key lookups мелких rows.\n\n\
                     **Tuning:**\n\
                     - `ORDER BY` — outer partition key (часто timestamp + dimension)\n\
                     - `PARTITION BY toYYYYMM(date)` — manageable parts\n\
                     - **TTL** — `TTL date + INTERVAL 90 DAY DELETE` для retention\n\
                     - Profile: `SELECT * FROM system.query_log WHERE query LIKE '%table%'`".into() },
                // ── Observability ─────────────────────────────────────
                Snippet { key: "prom".into(), title: "Prometheus + Alertmanager — основное".into(), body:
                    "**Архитектура:** Pull-based — Prometheus сам ходит за метриками на `/metrics` endpoint targets (полная противоположность InfluxDB push).\n\n\
                     **Service discovery:** static, file_sd, kubernetes_sd, consul_sd, ec2_sd. Targets находятся автоматом.\n\n\
                     **PromQL базовые:**\n\
                     - `rate(http_requests_total[5m])` — qps в last 5 min\n\
                     - `histogram_quantile(0.99, sum by(le) (rate(latency_bucket[5m])))` — p99\n\
                     - `up{job=\"api\"} == 0` — target down\n\
                     - `avg by(instance) (node_cpu_seconds_total{mode!=\"idle\"})` — CPU\n\n\
                     **Recording rules:** pre-compute expensive queries → быстрые dashboards.\n\n\
                     **Alertmanager:**\n\
                     - Group by `alertname, severity` — один email на 50 firing alerts\n\
                     - Inhibition: critical inhibits warning (на той же машине)\n\
                     - Silence: maintenance window\n\
                     - Routes: разные команды → разные channels (PagerDuty/Slack/email)\n\n\
                     **Retention:** local 15d default. Для long-term — Thanos / Cortex / VictoriaMetrics / Mimir.".into() },
                Snippet { key: "grafana".into(), title: "Grafana — dashboards + alerting do/don't".into(), body:
                    "**Dashboard design:**\n\
                     - **USE method** для resource-based (CPU/Memory/Disk/Net): Utilization, Saturation, Errors\n\
                     - **RED method** для service-based (HTTP/gRPC): Rate, Errors, Duration\n\
                     - Не делай 50-панельный «full overview». Лучше 3 dashboards: overview / drill-down / debug\n\
                     - Время в верхнем углу, variables в panel-affecting toolbar\n\n\
                     **Variables (templating):**\n\
                     - `$cluster`, `$namespace`, `$pod` — каскадные queries\n\
                     - `__rate_interval` — built-in, sane default для `rate()`\n\n\
                     **Alerting (Grafana 9+ unified alerting):**\n\
                     - Multi-dimensional (одно правило → много альертов по labels)\n\
                     - Связывай alert с dashboard через annotation `runbook_url`\n\
                     - `for: 5m` — пожар должен гореть 5 мин, иначе flapping\n\
                     - Notification policy = маршрутизация (как Alertmanager routes)\n\n\
                     **Anti-patterns:**\n\
                     - Alert per pod restart — false positives, used to be CrashLoopBackOff better\n\
                     - Single-value metric «is service alive» — лучше `up == 0 for 1m`\n\
                     - Hardcoded thresholds — % CPU зависит от размера node, лучше anomaly detection".into() },
                Snippet { key: "logs".into(), title: "Logging stack — ELK vs Loki vs ClickHouse".into(), body:
                    "**ELK (Elasticsearch + Logstash + Kibana):**\n\
                     - Full-text search, mature ecosystem\n\
                     - Дорогой по RAM/диску (inverted index на каждое поле)\n\
                     - Сложный operational toll (cluster master split-brain, shard rebalance)\n\
                     - Используй когда нужен **поиск по содержимому**\n\n\
                     **Loki (Grafana Labs):**\n\
                     - **Индексирует только labels**, не содержимое — дёшево\n\
                     - LogQL syntax напоминает PromQL\n\
                     - Идеален для **K8s + Prometheus stack** (одни labels)\n\
                     - Slower для grep по content больших volumes\n\
                     - Storage backend = S3/GCS, ~10× дешевле ES\n\n\
                     **ClickHouse:**\n\
                     - Топ по скорости aggregations\n\
                     - Materialized views для pre-computed metrics-from-logs\n\
                     - Используется Cloudflare, Uber, Datadog внутри\n\
                     - Steeper learning curve, нет нативного UI (но Grafana plugin есть)\n\n\
                     **Шиппинг:**\n\
                     - **Fluent Bit** — lightweight, K8s native, C-based\n\
                     - **Vector** (Datadog) — Rust, more flexible transforms\n\
                     - **Fluentd** — старая школа, Ruby, медленнее\n\
                     - **Promtail** — официальный шиппер для Loki".into() },
                Snippet { key: "trace".into(), title: "Distributed tracing — Jaeger / Tempo / OpenTelemetry".into(), body:
                    "**Зачем:** проследить один запрос через 10+ микросервисов. См. где p99 latency, где error.\n\n\
                     **Концепции:**\n\
                     - **Trace** = вся цепочка (один user request)\n\
                     - **Span** = одна операция (HTTP call, DB query). Имеет start_time, duration, parent_span_id\n\
                     - **Context propagation** — trace_id передаётся через `traceparent` header (W3C) или старый `X-B3-*` (Zipkin)\n\n\
                     **OpenTelemetry (стандарт):**\n\
                     - SDK для каждого языка → отправляет в **OTel Collector** → дальше в Jaeger/Tempo/Datadog\n\
                     - Auto-instrumentation для популярных libs (HTTP servers, gRPC, DB drivers)\n\
                     - Заменил OpenTracing + OpenCensus\n\n\
                     **Backends:**\n\
                     - **Jaeger** — старший, full-featured, in-memory или Cassandra/ES backend\n\
                     - **Tempo** (Grafana) — cheap storage в S3, integration с Loki/Mimir\n\
                     - **Zipkin** — самый старый, простой\n\n\
                     **Sampling:**\n\
                     - **Head-based** (% sampling прямо в SDK) — простой, тебе может не повезти не зацепить incident\n\
                     - **Tail-based** (в Collector, по error/latency) — дороже но «правильнее»".into() },
                // ── CI/CD ─────────────────────────────────────────────
                Snippet { key: "deploy".into(), title: "Deploy strategies — blue/green vs canary vs rolling".into(), body:
                    "**Rolling update (default K8s Deployment):**\n\
                     - Постепенно заменяем N pods, `maxSurge` + `maxUnavailable`\n\
                     - Плюс: простой, нет дополнительной инфры\n\
                     - Минус: смешанный traffic на старую+новую версии, hard rollback (нужен обратный rolling)\n\n\
                     **Blue/Green:**\n\
                     - Поднимаем целиком новый «green» environment\n\
                     - Переключаем traffic через LB / Ingress switch — атомарно\n\
                     - Плюс: instant rollback (вернуть LB обратно)\n\
                     - Минус: 2× ресурсов\n\n\
                     **Canary:**\n\
                     - 1% → 10% → 50% → 100% постепенно\n\
                     - Метрики (error rate, latency, business KPIs) — автоматический abort\n\
                     - Tools: **Argo Rollouts**, **Flagger** (Flux)\n\
                     - Лучший для prod high-traffic\n\n\
                     **A/B testing** ≠ canary:\n\
                     - A/B = product experiment (feature change)\n\
                     - Canary = infra deploy (same feature, новая версия binary)\n\n\
                     **Feature flags** (LaunchDarkly, Unleash) — orthogonal: код задеплоен, фича скрыта.".into() },
                Snippet { key: "argo".into(), title: "GitOps + ArgoCD — push vs pull деплой".into(), body:
                    "**GitOps принципы (Weaveworks):**\n\
                     1. Git = source of truth для всего (manifests, configs)\n\
                     2. Декларативные манифесты (K8s YAML, Terraform, Crossplane)\n\
                     3. Auto-sync: agent в cluster постоянно сверяет реальное состояние с Git\n\
                     4. Pull-based — cluster сам тянет, не CI пушит\n\n\
                     **ArgoCD:**\n\
                     - **Application** = (Git repo + path) → (K8s cluster + namespace)\n\
                     - Auto-sync polling 3min или webhook trigger\n\
                     - Sync waves для ordered deploy (CRD → operator → instance)\n\
                     - UI показывает diff между Git и live, drift detection\n\n\
                     **Argo Rollouts** (отдельный controller):\n\
                     - Заменяет Deployment на Rollout (CRD)\n\
                     - Canary / Blue-Green с analysis templates (Prometheus query → abort)\n\n\
                     **Flux v2** (CNCF, конкурент):\n\
                     - Более модульный (Source + Kustomize + Helm controllers)\n\
                     - Лучше для multi-tenancy и multi-cluster\n\
                     - Менее красивый UI\n\n\
                     **Tradeoff push vs pull:**\n\
                     - Push (CI → cluster) — нужны cluster creds в CI, проще для одного env\n\
                     - Pull (GitOps) — credentials только у agent, лучше security boundary".into() },
                Snippet { key: "ci".into(), title: "CI pipeline — что должно быть на каждом step".into(), body:
                    "**Стандартный pipeline для backend (порядок важен):**\n\n\
                     1. **Lint + format check** — fail fast, < 30s. golangci-lint, ruff, eslint\n\
                     2. **Unit tests** — параллелизуй, coverage gate (≥70% обычно sane)\n\
                     3. **Build** — cache dependencies. Docker multi-stage для slim images\n\
                     4. **Security scan:** image (Trivy/Grype), secrets (gitleaks), SAST (semgrep)\n\
                     5. **Integration tests** — нужны живые БД (Testcontainers, docker-compose)\n\
                     6. **Push artifact** — image в registry, tag = git SHA (не `latest`)\n\
                     7. **Deploy to dev** — auto на main branch\n\
                     8. **E2E tests** — Playwright/Cypress против dev env\n\
                     9. **Deploy to staging/prod** — manual approval или canary auto\n\n\
                     **Принципы:**\n\
                     - **Каждый PR проходит pipeline до build** (минимум)\n\
                     - **Каждый commit на main** = автоматический deploy в dev\n\
                     - **Артефакт неизменен** — образ собран один раз, проходит env-to-env\n\
                     - **Pipeline = код** (Jenkinsfile, .github/workflows, .gitlab-ci.yml) — в репо, ревьювится\n\n\
                     **Cache:** Docker layer cache (BuildKit `--cache-from`), npm/cargo/pip cache в `~/.cache`. Может ускорить 5-10×.".into() },
                Snippet { key: "secrets-ci".into(), title: "Secrets в CI/CD — где НЕ хранить + где можно".into(), body:
                    "**Где НЕЛЬЗЯ:**\n\
                     - В код / `.env` файлах в репо (даже private!)\n\
                     - В Dockerfile (`ENV API_KEY=...`) — попадёт в image layer навсегда\n\
                     - В CI job logs — `set -x` в shell или `echo $TOKEN` спалит\n\
                     - В deployment manifests как plaintext\n\n\
                     **Где можно (по убыванию security):**\n\
                     1. **External vault** (HashiCorp Vault, AWS Secrets Manager) — dynamic secrets (Vault выдаёт DB cred на 1 час)\n\
                     2. **Sealed Secrets / SOPS** — зашифрованные YAML в Git, расшифровка только в cluster\n\
                     3. **OIDC federation** — CI получает короткоживущий token от AWS/GCP по trust relationship (без долгоживущих keys)\n\
                     4. **GitHub Actions secrets / GitLab CI variables** — масked в логах, но всё ещё доступно maintainer'у\n\
                     5. **K8s Secret** (encrypted-at-rest!) — для уже задеплоенного app\n\n\
                     **Best practices:**\n\
                     - **Rotation** — short TTL + автоматическая ротация\n\
                     - **Audit log** — кто/когда читал каждый secret\n\
                     - **Least privilege** — отдельный SA на каждый workload\n\
                     - **Не передавай secrets через args** (видны в `ps`) — только env или mounted files".into() },
                // ── Cloud ─────────────────────────────────────────────
                Snippet { key: "aws-vpc".into(), title: "AWS VPC — subnets / routing / connectivity".into(), body:
                    "**Структура VPC:**\n\
                     - **VPC** = appname + CIDR (10.0.0.0/16)\n\
                     - **Subnets** = AZ-specific (10.0.1.0/24 в us-east-1a). Public vs Private отличается route table\n\
                     - **Route table:** Public ─→ IGW (Internet Gateway). Private ─→ NAT Gateway (для outbound) или Endpoints\n\
                     - **VPC Endpoints** — приватный доступ к AWS services (S3, DynamoDB Gateway endpoints бесплатные)\n\n\
                     **Связь между VPC:**\n\
                     - **VPC Peering** — point-to-point, transitive routing НЕТ\n\
                     - **Transit Gateway** — hub-and-spoke, scales до тысяч VPCs, поддерживает SD-WAN\n\
                     - **PrivateLink** — service exposure без peering (cross-account SaaS)\n\n\
                     **Security:**\n\
                     - **Security Group** = stateful firewall (Pod-level), на ENI\n\
                     - **NACL** = stateless ACL (subnet-level), и in и out нужны\n\
                     - **Flow Logs** — pcap-style traffic log в S3 или CloudWatch\n\n\
                     **Подводные камни:**\n\
                     - 5 SG per ENI default — можно увеличить через quota\n\
                     - NAT Gateway = $0.045/hr + $0.045/GB processed — много трафика = дорого. Vpce S3 спасает\n\
                     - IPv6 поддерживается, но Dual-stack настраивать руками".into() },
                Snippet { key: "aws-iam".into(), title: "AWS IAM — Users / Roles / Policies — кратко".into(), body:
                    "**4 главных объекта:**\n\
                     - **User** — человек/external system с долговременными credentials\n\
                     - **Group** — набор политик на множество users\n\
                     - **Role** — переключаемая identity (assume-role), short-lived creds. Используй для EC2/Lambda/cross-account\n\
                     - **Policy** — JSON document с Statements (`Effect`, `Action`, `Resource`, `Condition`)\n\n\
                     **Принципы:**\n\
                     - **НЕ ИСПОЛЬЗОВАТЬ root account** — только для billing + начальный setup\n\
                     - **НЕ хранить access keys** в EC2 — Instance Profile (Role) даёт STS creds автоматически\n\
                     - **Least privilege** — `Action: \"s3:GetObject\", Resource: \"arn:aws:s3:::my-bucket/*\"` (не `s3:*`)\n\
                     - **MFA на всё** — особенно root и IAM с привилегиями\n\n\
                     **Conditions:**\n\
                     - `aws:SourceIp` — restrict by IP\n\
                     - `aws:MultiFactorAuthPresent` — требовать MFA для critical actions\n\
                     - `aws:RequestTag/Project` — tag-based authorization\n\n\
                     **Permission boundary** — max permissions для созданных пользователем roles (для delegating IAM admin developers).".into() },
                Snippet { key: "s3".into(), title: "S3 — consistency, storage classes, типичные паттерны".into(), body:
                    "**Consistency (с 2020):** strong read-after-write для всех ops (включая overwrites + deletes). Раньше eventual для overwrites.\n\n\
                     **Storage classes:**\n\
                     - **Standard** — default, multi-AZ, hot\n\
                     - **Intelligent-Tiering** — auto переключение по access patterns ($)\n\
                     - **Standard-IA / One Zone-IA** — infrequent access, дёшево read но retrieval fee\n\
                     - **Glacier Instant Retrieval** — milliseconds retrieval, минимум 90 дней\n\
                     - **Glacier Flexible / Deep Archive** — часы/дни retrieval, минимум 90/180 дней. Архив compliance\n\n\
                     **Lifecycle policies:** auto-transition `Standard → IA → Glacier → Delete` по возрасту objects.\n\n\
                     **Производительность:**\n\
                     - 3500 PUT/COPY/POST/DELETE per second per prefix\n\
                     - 5500 GET/HEAD per second per prefix\n\
                     - Prefix sharding для high-throughput (`/2024/01/01/...` vs `<hash>/...`)\n\
                     - Multipart upload для >100 MB файлов (parallelism + resume)\n\n\
                     **Security:**\n\
                     - **Bucket policy** + **ACL** + **Block Public Access** (последнее — fail-safe)\n\
                     - **SSE-S3 / SSE-KMS** — encryption at rest, KMS даёт audit log\n\
                     - **Object Lock + WORM** — compliance (нельзя удалить N дней)\n\
                     - **Versioning** + **MFA Delete** — защита от ransomware/accidental delete".into() },
                // ── Containers ────────────────────────────────────────
                Snippet { key: "docker".into(), title: "Docker — layers, multi-stage, dockerfile best practices".into(), body:
                    "**Layers:**\n\
                     - Каждая `RUN` / `COPY` / `ADD` создаёт новый layer\n\
                     - Layers immutable, кешируются → меняешь нижний layer = пересобираешь всё выше\n\
                     - **Order matters:** редко-меняющиеся (`apt install`) ВВЕРХУ, часто-меняющиеся (`COPY src/`) ВНИЗУ\n\n\
                     **Multi-stage build:**\n\
                     ```dockerfile\n\
                     FROM golang:1.22 AS builder\n\
                     WORKDIR /src\n\
                     COPY go.mod go.sum ./\n\
                     RUN go mod download\n\
                     COPY . .\n\
                     RUN CGO_ENABLED=0 go build -o /app\n\n\
                     FROM gcr.io/distroless/static:nonroot\n\
                     COPY --from=builder /app /app\n\
                     ENTRYPOINT [\"/app\"]\n\
                     ```\n\
                     Финальный image ~10 MB вместо 800 MB.\n\n\
                     **Best practices:**\n\
                     - `USER nonroot` — не root внутри контейнера\n\
                     - `HEALTHCHECK` — Docker / orchestrator знает что app живой\n\
                     - `.dockerignore` — не пихай `.git`, `node_modules` в context\n\
                     - **Don't run as PID 1 без init** — `tini` или `--init` для signal handling\n\
                     - **Pin versions:** `python:3.11.7-slim`, не `python:latest`\n\
                     - **Cache mounts** (BuildKit): `RUN --mount=type=cache,target=/root/.cache/go-build go build` — ускоряет 5-10×".into() },
                // ── Security ──────────────────────────────────────────
                Snippet { key: "oauth2".into(), title: "OAuth 2.0 / OIDC — потоки + когда какой".into(), body:
                    "**Базовые роли:**\n\
                     - **Resource Owner** — user\n\
                     - **Client** — приложение (web, mobile, CLI)\n\
                     - **Authorization Server** — выдаёт tokens (Auth0, Keycloak, Okta)\n\
                     - **Resource Server** — API, валидирует tokens\n\n\
                     **Flows (выбирай по типу client):**\n\n\
                     - **Authorization Code + PKCE** — для web/mobile/SPA (modern default). Browser → auth → exchange code на token.\n\
                     - **Client Credentials** — machine-to-machine (cron job, microservice). Только client_id+secret, нет user.\n\
                     - **Device Code** — для CLI без браузера / smart TV. Показывает URL+code на одном устройстве, login на другом.\n\
                     - **Refresh Token** — продление access_token без re-login. Храни SECURE (httpOnly cookie или secure storage).\n\n\
                     **❌ Deprecated:** Implicit (XSS-prone), Resource Owner Password Credentials (нарушает разделение ответственности).\n\n\
                     **OIDC vs OAuth:** OIDC = OAuth 2.0 + identity layer. Возвращает **id_token** (JWT с claims о user). OAuth = authorization (access), OIDC = authentication (who).\n\n\
                     **JWT валидация:** проверять signature, `iss`, `aud`, `exp`. Public key через JWKS endpoint (`.well-known/jwks.json`).".into() },
                Snippet { key: "owasp".into(), title: "OWASP Top 10 (2021) — что чаще всего ломают".into(), body:
                    "1. **Broken Access Control** — `/admin` без RBAC, IDOR (`/users/123` → меняешь на 124), missing function-level checks\n\
                     2. **Cryptographic Failures** — секреты в логах, weak ciphers (MD5/SHA1 для passwords), no TLS\n\
                     3. **Injection** — SQL/NoSQL/Command/LDAP. **Parameterized queries**, ORM. Никогда string concat!\n\
                     4. **Insecure Design** — отсутствие threat modeling. Например, password reset → token в URL → log → утечка\n\
                     5. **Security Misconfiguration** — default creds, debug mode in prod, verbose error pages, открытые порты\n\
                     6. **Vulnerable Components** — устаревшие libs. Tools: `npm audit`, `pip-audit`, Dependabot, Trivy, Snyk\n\
                     7. **Identification/Auth Failures** — weak passwords allowed, нет rate limit на login, predictable session IDs\n\
                     8. **Software/Data Integrity Failures** — unsigned updates, npm/pip packages from random source, CI без integrity check\n\
                     9. **Security Logging Failures** — не логировать auth events; **или** логировать sensitive data\n\
                     10. **SSRF** — Server-Side Request Forgery. App fetches `?url=...` без validation → атакующий достаёт `http://169.254.169.254/` (metadata)\n\n\
                     **Defense in depth:** WAF + secure code + monitoring + patch cadence. Никогда **одна** мера.".into() },
                // ── SRE ───────────────────────────────────────────────
                Snippet { key: "capacity".into(), title: "Capacity planning — формулы + что учитывать".into(), body:
                    "**Базовый расчёт:**\n\n\
                     `Required capacity = peak_qps × avg_response_time × safety_factor`\n\n\
                     Пример: 10k qps peak × 50ms response × 1.5 safety = 750 concurrent requests. При 100 RPS/instance → 8 instances.\n\n\
                     **Что учитывать:**\n\
                     - **Headroom** — никогда 100% utilization. SRE practice: 60-70% peak\n\
                     - **Growth** — Q-over-Q business metric forecast (если product растёт 20% QoQ — capacity тоже)\n\
                     - **Failover scenario** — если одна AZ упала, оставшиеся должны вынести 100%. Значит 3 AZ × 50% normal load = 150% capacity\n\
                     - **Burst pattern** — peak/avg ratio. Black Friday = 10× normal. Что делать?\n\
                     - **Resource limits** не только CPU/RAM:\n\
                       - DB connections (PgBouncer max?)\n\
                       - File descriptors (ulimit)\n\
                       - Port range (ephemeral ports)\n\
                       - SNAT ports на NAT gateway (AWS limit 55k per public IP)\n\n\
                     **Load testing:**\n\
                     - `k6`, `locust`, `wrk`, `vegeta` — gradual ramp до breakpoint\n\
                     - **Найди где деградирует** (response time / error rate / queue depth) — это твой real capacity, не теоретический.\n\
                     - **Chaos engineering** (Gremlin, Litmus) — что если узкое место упадёт?".into() },
                Snippet { key: "runbook".into(), title: "Runbook — структура для on-call".into(), body:
                    "**Каждый алерт = runbook с linkable URL** в Alert annotations.\n\n\
                     **Структура runbook:**\n\n\
                     1. **Алерт name + summary** (что значит этот алерт)\n\
                     2. **Severity:** SEV1 (page) / SEV2 (slack) / SEV3 (ticket)\n\
                     3. **First actions** (≤5 шагов, конкретные команды):\n\
                        - `kubectl logs deployment/api -n prod --tail=200`\n\
                        - `curl https://api.example.com/health`\n\
                        - dashboard URL\n\
                     4. **Common causes** (с диагностикой каждой):\n\
                        - DB connection pool exhausted → `SELECT count(*) FROM pg_stat_activity`\n\
                        - Upstream slow → `grep upstream_response_time access.log`\n\
                     5. **Mitigation** (что делать, в порядке от safest):\n\
                        - Auto-restart pod\n\
                        - Scale up replicas\n\
                        - Failover to standby\n\
                        - Rollback last deploy\n\
                     6. **Escalation:** когда призывать оригинального owner / senior\n\
                     7. **Post-mortem template link**\n\n\
                     **Принципы:**\n\
                     - **Каждый алерт должен иметь runbook** (или: алерт удалить)\n\
                     - **Junior on-call** должен пройти runbook без помощи\n\
                     - **Update после каждого incident** — что нового узнали? Add to runbook.\n\
                     - Версионирование в Git, ревью изменений\n\
                     - Test раз в квартал — chaos drill «прокликай как на page»".into() },
                Snippet { key: "errorbudget".into(), title: "Error budget — как использовать на практике".into(), body:
                    "**Базовая формула:**\n\
                     - SLO `99.9% availability` → budget `0.1%` = 43.2 min downtime/month\n\
                     - SLO `99.95%` → 21.6 min/month\n\
                     - SLO `99.99%` (\"four nines\") → 4.3 min/month — **серьёзная стоимость**\n\n\
                     **Что значит \"бюджет сгорел\":**\n\
                     - **Stop feature releases** — freezing deploys на N дней\n\
                     - Focus engineers на reliability: chaos drills, runbooks, alerting тuning\n\
                     - Не «давайте увеличим SLO до 99.99%» — это игнорирует реальность\n\n\
                     **Что значит \"бюджет в запасе\":**\n\
                     - **Take risks:** deploy чаще, agressive canary, experimental features\n\
                     - Plan maintenance windows — не отнимай у бюджета unplanned outages\n\
                     - Run intentional failure tests (Gameday)\n\n\
                     **Multi-window burn rate (Google SRE book):**\n\
                     - Slow burn: за 6 часов сгорело 5% бюджета → page on-call\n\
                     - Fast burn: за 5 минут сгорело 2% → page + escalate\n\
                     - Avoids paging on transient spike, но не upset for sustained issue\n\n\
                     **Дискуссия с PM:**\n\
                     - SLO = договор между **infra и product** team\n\
                     - Если product хочет deploy 10× в день → нужен ОБЪЕКТИВНЫЙ budget tracker\n\
                     - Нет budget tracker = SLO = wishful thinking".into() },
                // ── Microservices ─────────────────────────────────────
                Snippet { key: "saga".into(), title: "Saga pattern — распределённые транзакции".into(), body:
                    "**Проблема:** один бизнес-процесс трогает 3 сервиса (Order → Payment → Inventory). 2PC дорогой и хрупкий.\n\n\
                     **Saga = последовательность local транзакций с compensating actions.**\n\n\
                     **Choreography (decentralised):**\n\
                     - Каждый сервис emit event, остальные subscribe\n\
                     - OrderCreated → Payment subscribes → reserves\n\
                     - PaymentReserved → Inventory subscribes → reserves\n\
                     - InventoryReserved → Order subscribes → finalises\n\
                     - **Плюс:** loose coupling, нет центральной точки отказа\n\
                     - **Минус:** сложно дебажить (где мы в saga?), implicit dependency graph\n\n\
                     **Orchestration (central coordinator):**\n\
                     - Saga orchestrator (state machine) вызывает services по очереди\n\
                     - Tools: Temporal, Camunda, AWS Step Functions\n\
                     - **Плюс:** explicit flow, visualization, retry/timeout встроены\n\
                     - **Минус:** SPOF coordinator (HA нужен), tight coupling от orchestrator\n\n\
                     **Compensating actions ОБЯЗАТЕЛЬНЫ:**\n\
                     - Payment failed → emit OrderCancelled → Inventory releases reservation\n\
                     - **НЕ ВСЕ операции откатываются** — отправил email? Compensate = «sorry» email\n\n\
                     **Idempotency** критична — message broker может deliver дважды.\n\
                     **Outbox pattern** — атомарность \"DB write + event publish\".".into() },
                Snippet { key: "mesh".into(), title: "Service mesh — Istio / Linkerd, когда нужен".into(), body:
                    "**Что делает mesh:** sidecar (Envoy/proxy) рядом с каждым app handle:\n\
                     - **mTLS** автоматически между всеми services (zero-trust networking)\n\
                     - **Traffic management** — canary, A/B, retries, timeouts, circuit breakers\n\
                     - **Observability** — automatic metrics/traces без правок app кода\n\
                     - **Policy** — кто может звать кого (authorization)\n\n\
                     **Istio:**\n\
                     - Feature-rich, complex. Envoy data plane + Istiod control plane\n\
                     - VirtualService / DestinationRule / Gateway — CRDs\n\
                     - Steep learning curve, but unmatched flexibility\n\
                     - Ambient mode (новый) — без sidecars, ztunnel + waypoint\n\n\
                     **Linkerd:**\n\
                     - Simpler, Rust-based proxy (быстрее, меньше памяти чем Envoy)\n\
                     - Лучше для smaller / starting teams\n\
                     - Менее feature-богат\n\n\
                     **Когда НЕ нужен:**\n\
                     - <10 микросервисов — overhead не оправдан\n\
                     - Если auth/TLS уже делается на app level (libraries)\n\
                     - Один namespace — простой Network Policy достаточно\n\n\
                     **Когда нужен:**\n\
                     - 50+ services, multi-team\n\
                     - Compliance: \"all traffic encrypted\"\n\
                     - Cross-cluster / multi-region routing\n\
                     - Платформенная команда стандартизирует observability".into() },
                Snippet { key: "circuit".into(), title: "Circuit breaker + retry — паттерны устойчивости".into(), body:
                    "**Circuit breaker состояния:**\n\
                     - **Closed** — нормальное прохождение запросов\n\
                     - **Open** — открыт после N failures, request fails fast БЕЗ обращения к upstream\n\
                     - **Half-Open** — после cooldown пробует ОДИН запрос. Success → Closed. Fail → Open\n\n\
                     **Параметры:**\n\
                     - `failure_threshold = 50%` за окно 10s\n\
                     - `request_volume_threshold = 20` (минимум для статистики)\n\
                     - `sleep_window = 5s` (Open → Half-Open delay)\n\n\
                     **Retry правила:**\n\
                     - **Exponential backoff with jitter:** `delay = min(cap, base * 2^attempt) + rand(0, base)`\n\
                     - **НЕ retry на 4xx** (твоя ошибка, не upstream)\n\
                     - **Retry на:** 502, 503, 504, network timeout, connection refused\n\
                     - **Retry budget** — макс N retries за окно (не битьём в стенку весь pool)\n\
                     - **Idempotency!** Не retry POST без идемпотентного key\n\n\
                     **Timeout каскад:**\n\
                     - Client timeout (e.g. 30s) > сумма всех downstream timeouts + retries\n\
                     - Иначе retry сработает после того как client уже отвалился\n\n\
                     **Libraries:**\n\
                     - **resilience4j** (Java), **polly** (.NET), **tenacity** (Python)\n\
                     - **Envoy** делает это в service mesh без кода\n\
                     - **Hystrix** deprecated — see resilience4j".into() },
                // ── Message Queues ────────────────────────────────────
                Snippet { key: "kafka".into(), title: "Kafka — partitions, consumer groups, semantics".into(), body:
                    "**Базовые концепции:**\n\
                     - **Topic** = log of messages\n\
                     - **Partition** = ordered immutable sequence (parallel unit)\n\
                     - **Offset** = position в partition (consumer tracks)\n\
                     - **Replication factor** = N брокеров хранят копию (typical 3)\n\
                     - **Producer key** → hash(key) % partitions = always same partition (ordering per key)\n\n\
                     **Consumer groups:**\n\
                     - Один consumer group получает каждое сообщение РАЗ\n\
                     - Partitions распределяются между consumers в группе\n\
                     - `# consumers ≤ # partitions` (лишние idle)\n\
                     - **Rebalance** при join/leave — pause traffic\n\n\
                     **Delivery semantics:**\n\
                     - **At-most-once** — commit offset BEFORE process → может потерять при crash\n\
                     - **At-least-once** (default) — process THEN commit → может дублировать\n\
                     - **Exactly-once** — `transactional.id` + idempotent producer + `isolation.level=read_committed` consumer\n\n\
                     **Tuning производительности:**\n\
                     - Producer: `batch.size=64KB`, `linger.ms=10` — batching\n\
                     - `compression.type=lz4` (хороший trade-off speed/ratio)\n\
                     - `acks=all` (durability) vs `acks=1` (throughput) vs `acks=0` (fire-and-forget)\n\
                     - Consumer: `max.poll.records=500`, `fetch.min.bytes=1MB`\n\n\
                     **Retention:** `retention.ms=7d` (time) или `retention.bytes=10GB` (size). Compacted topic = только последнее значение per key.".into() },
                Snippet { key: "rabbit".into(), title: "RabbitMQ — exchanges, queues, когда vs Kafka".into(), body:
                    "**4 типа exchanges:**\n\
                     - **Direct** — routing key == binding key (exact match)\n\
                     - **Topic** — pattern match (`logs.*.error`, wildcard)\n\
                     - **Fanout** — broadcast в все bound queues (ignores routing key)\n\
                     - **Headers** — match по message headers (редко используется)\n\n\
                     **Queue types:**\n\
                     - **Classic** — single-node, replicas через mirroring (deprecated in 4.0)\n\
                     - **Quorum** (recommended) — Raft consensus, HA, persistent\n\
                     - **Streams** (3.9+) — Kafka-like append-only log\n\n\
                     **RabbitMQ vs Kafka:**\n\
                     - **RabbitMQ:** flexible routing, per-message ACK, push model, lower latency для small messages, лучше для task queues / job dispatch\n\
                     - **Kafka:** high throughput, replay-able log, partitioned scale, event streaming / log aggregation\n\n\
                     **Делегирование выбора:**\n\
                     - \"Worker pool делает email-отправку\" → RabbitMQ + work queue\n\
                     - \"Event sourcing 1M events/sec\" → Kafka\n\
                     - \"Pub/sub микросервисов\" → оба подходят, выбирай team familiarity\n\
                     - \"Order processing, последовательность важна per-customer\" → Kafka (partition by customer_id)\n\n\
                     **Anti-patterns:** RabbitMQ как long-term storage (TTL maxes out), Kafka как RPC bus (overkill).".into() },
                // ── Performance / Caching ─────────────────────────────
                Snippet { key: "cache".into(), title: "Cache strategies — write-through / -back / -around".into(), body:
                    "**Read patterns:**\n\
                     - **Cache-aside** (lazy loading): app сам проверяет cache → miss → fetch DB → populate cache. Простой, fault-tolerant\n\
                     - **Read-through:** cache provider сам fetches DB на miss. Cleaner code, но cache становится SPOF\n\n\
                     **Write patterns:**\n\
                     - **Write-through:** write идёт в cache И в DB sync. Slow writes, fresh cache\n\
                     - **Write-back / write-behind:** write только в cache, async flush в DB. Fast writes, риск потерь\n\
                     - **Write-around:** write в DB, cache игнорируется. Cache miss на следующий read\n\n\
                     **Eviction:**\n\
                     - **LRU** — Least Recently Used (default Redis `allkeys-lru`)\n\
                     - **LFU** — Least Frequently Used (Redis `allkeys-lfu`, лучше для stable access patterns)\n\
                     - **FIFO** — простой queue, плохо для cache\n\
                     - **Random** — surprisingly competitive\n\
                     - **TTL** — time-based, complementary к eviction\n\n\
                     **Invalidation (\"two hardest problems in CS\"):**\n\
                     - **TTL** — простой но stale data до expiry\n\
                     - **Explicit invalidate** — write path удаляет cache key. Хрупкий (легко забыть)\n\
                     - **Event-driven** — DB change → publish → cache subscribers invalidate\n\
                     - **Versioned keys** — `user:42:v3` — release new version = effectively new cache\n\n\
                     **Cache stampede:** thundering herd когда expired key fetched 1000× одновременно. Lock + double-check или probabilistic early refresh.".into() },
                // ── Search ────────────────────────────────────────────
                Snippet { key: "es".into(), title: "Elasticsearch basics — index, mapping, query".into(), body:
                    "**Inverted index:** для каждого term → список documents где он встречается. Это база full-text search.\n\n\
                     **Иерархия:**\n\
                     - **Cluster** ⊃ **Indices** ⊃ **Shards** ⊃ **Segments** (Lucene level)\n\
                     - **Document** = JSON object с auto-assigned `_id`\n\n\
                     **Mapping** (= schema):\n\
                     - `keyword` — exact match (фильтр, aggregation)\n\
                     - `text` — full-text, analyzed (stems, lowercase, stop words)\n\
                     - Часто хочешь оба: `\"name\": {\"type\":\"text\", \"fields\":{\"keyword\":{\"type\":\"keyword\"}}}`\n\
                     - **Dynamic mapping** — ES угадывает типы. Опасно в prod, делай explicit\n\n\
                     **Query DSL основное:**\n\
                     - `match` — full-text query (analyze + search)\n\
                     - `term` — exact match (НЕ для text fields — будет искать analyzed token)\n\
                     - `bool { must, should, must_not, filter }` — composite\n\
                     - `aggs` — Elasticsearch's group by + analytics\n\n\
                     **Sharding:**\n\
                     - Number of shards задаётся при создании index, **не меняется** (нужен reindex)\n\
                     - Replicas меняются live (`PUT /_settings`)\n\
                     - Rule of thumb: shard size 10-50 GB. 200 shards on small index = wasted overhead\n\n\
                     **Anti-patterns:**\n\
                     - Использовать как primary DB (нет transactions, eventual consistency)\n\
                     - Indexing 10M docs за раз без bulk API + refresh tuning\n\
                     - `wildcard` queries (`*foo*`) на больших indexes — full scan".into() },
                // ── Streaming / ML-Ops ────────────────────────────────
                Snippet { key: "mlops".into(), title: "ML-Ops basics — model serving + monitoring".into(), body:
                    "**ML lifecycle:**\n\
                     1. **Data ingestion + validation** (Great Expectations, TFDV)\n\
                     2. **Feature engineering** — feature store (Feast, Tecton) для re-use\n\
                     3. **Training** — track experiments (MLflow, W&B), versioned data (DVC)\n\
                     4. **Validation** — accuracy / fairness / robustness checks\n\
                     5. **Serving** (см. ниже)\n\
                     6. **Monitoring** — data drift, model drift, business metrics\n\n\
                     **Serving patterns:**\n\
                     - **Batch:** scheduled job предсказывает на all customers за ночь, results в DB\n\
                     - **Real-time online:** REST/gRPC endpoint, low latency (<100ms p99)\n\
                     - **Streaming:** Kafka → consumer применяет модель → новый topic\n\
                     - **Edge:** TFLite / ONNX / CoreML на устройстве\n\n\
                     **Tools:**\n\
                     - **TF Serving, TorchServe** — фреймворк-specific\n\
                     - **NVIDIA Triton** — multi-framework, GPU optimized\n\
                     - **BentoML, KServe** — Kubernetes-native, abstracts framework\n\
                     - **Seldon Core** — advanced (canary, A/B, explainers)\n\n\
                     **Monitoring (новые типы ошибок vs обычный app):**\n\
                     - **Data drift** — input distribution меняется (PSI / KL-divergence per feature)\n\
                     - **Concept drift** — relationship X→Y меняется\n\
                     - **Model performance в проде** — нужны ground-truth labels (delayed feedback)\n\
                     - **Shadow deployment** — новая модель работает рядом, results сравниваются offline".into() },
                // ── Diagnostic checklist ──────────────────────────────
                Snippet { key: "slow".into(), title: "«Сайт тормозит» — общий чеклист 5 минут".into(), body:
                    "**Step 1: где именно медленно** (узнать ДО digging):\n\
                     - DevTools Network tab → TTFB или waterfall?\n\
                     - Server-side timing (`Server-Timing` header) — DB / cache / template render?\n\
                     - APM trace (Datadog / New Relic / Jaeger) — какой span главный contributor?\n\n\
                     **Step 2: типичные подозреваемые:**\n\n\
                     **DB-related:**\n\
                     - Slow query (`pg_stat_statements`, `mysql slow_query_log`)\n\
                     - Connection pool exhausted (`pg_stat_activity` показывает 1000 idle)\n\
                     - Lock contention (long-running transaction)\n\
                     - Missing index (после deploy ALTER TABLE без индекса)\n\n\
                     **Cache-related:**\n\
                     - Cache miss rate взлетел (Redis: `INFO stats` → `keyspace_misses`)\n\
                     - Cache stampede после mass eviction\n\n\
                     **App-related:**\n\
                     - GC pause (Java: `-Xlog:gc*`, Node: `--inspect`)\n\
                     - CPU pegged (`top`, profiler)\n\
                     - Memory leak → swap → 10× slowdown\n\
                     - N+1 queries (ORM lazy loading)\n\n\
                     **External:**\n\
                     - Third-party API slow (logs upstream_response_time)\n\
                     - DNS resolution slow (resolver fails → app retries)\n\
                     - Network packet loss (mtr)\n\
                     - CDN cache miss → origin overload\n\n\
                     **Step 3: метрики глобально:**\n\
                     - Дашборд RED (Rate, Errors, Duration) — где аномалия?\n\
                     - Compare с baseline неделю назад\n\
                     - Recent deploys? Rollback if matches start time".into() },
                Snippet { key: "memleak".into(), title: "Memory leak debug — Linux + основные runtimes".into(), body:
                    "**Симптомы:** RAM растёт монотонно во времени, без plateau. После N часов — OOM или swap thrash.\n\n\
                     **Базовая диагностика:**\n\
                     - `ps aux --sort=-%mem | head` — кто жрёт\n\
                     - `cat /proc/<pid>/status | grep -E 'VmRSS|VmPeak'`\n\
                     - `smem -tk` — учитывает shared memory правильно\n\
                     - `pmap -x <pid>` — детальный breakdown\n\n\
                     **JVM (Java/Kotlin/Scala):**\n\
                     - **Heap dump:** `jcmd <pid> GC.heap_dump /tmp/heap.hprof`\n\
                     - **Анализ:** Eclipse MAT, VisualVM, IntelliJ profiler — ищи **dominator tree**\n\
                     - **Live analysis:** `jcmd <pid> VM.native_memory summary` (если NMT enabled)\n\
                     - Частые источники: ThreadLocal'ы, кеши без bound, classloader leaks\n\n\
                     **Node.js:**\n\
                     - `--inspect` flag → Chrome DevTools Memory tab → heap snapshot\n\
                     - **Compare 2 snapshots** — найти что появилось\n\
                     - Closure-related, EventEmitter listeners без `off()`, Promises держат references\n\n\
                     **Go:**\n\
                     - `import _ \"net/http/pprof\"` → `go tool pprof http://...:6060/debug/pprof/heap`\n\
                     - `top -cum`, `list <fn>`, `web` — flame graph\n\
                     - Goroutine leaks: `pprof/goroutine` — растут ли\n\n\
                     **Python:**\n\
                     - `tracemalloc.start()` → `tracemalloc.take_snapshot()` → diff\n\
                     - `objgraph.show_growth()` — что новых instances\n\
                     - `memory_profiler` decorator для line-level".into() },
                // ── Misc one-liners ───────────────────────────────────
                Snippet { key: "jvm".into(), title: "JVM tuning — флаги + GC выбор".into(), body:
                    "**Heap size:**\n\
                     - `-Xms4G -Xmx4G` — установи min=max чтоб JVM не resize'ил\n\
                     - **Container-aware:** `-XX:MaxRAMPercentage=75` (Java 10+, проще чем считать MB)\n\n\
                     **GC выбор (Java 17+):**\n\
                     - **G1GC** (default) — balance latency/throughput, default <=4 GB\n\
                     - **ZGC** (`-XX:+UseZGC`) — pause < 1ms, для latency-sensitive, поддерживает терабайтные heaps\n\
                     - **Shenandoah** (RedHat) — конкурент ZGC\n\
                     - **Parallel GC** — old-school, max throughput, длинные паузы. Для batch.\n\n\
                     **Observability:**\n\
                     - `-Xlog:gc*:file=/var/log/gc.log:time,uptime,level,tags` — structured GC log\n\
                     - **GCEasy.io** — paste log → визуализация\n\
                     - `jcmd <pid> GC.heap_info` — runtime heap state\n\n\
                     **JIT:**\n\
                     - **Tiered compilation** (default) — quick C1 → optimal C2\n\
                     - `-XX:+PrintCompilation` — что инлайнится / деоптимизируется\n\
                     - **GraalVM** — alternative JIT, иногда быстрее, иногда медленнее\n\n\
                     **Container gotchas:**\n\
                     - JVM до Java 10 не видел cgroup limits → heap > container memory → OOMKilled\n\
                     - **Java 10+:** `-XX:+UseContainerSupport` (default on)\n\
                     - CPU: `-XX:ActiveProcessorCount=N` если container limits < hostspecific".into() },
                Snippet { key: "git".into(), title: "Git advanced — rebase, bisect, reflog, hooks".into(), body:
                    "**Rebase vs merge:**\n\
                     - **Merge** — сохраняет история, делает merge commit. History видит \"когда был merged\"\n\
                     - **Rebase** — переносит твои commits на свежий main. Linear history\n\
                     - **Golden rule:** не rebase pushed branches которые юзают другие\n\n\
                     **Interactive rebase** (`git rebase -i HEAD~5`):\n\
                     - `pick / reword / edit / squash / drop` — clean up история перед PR\n\
                     - Полезно сводить 12 \"fix typo\" → 1 logical commit\n\n\
                     **`git bisect`** — найти когда баг introduced:\n\
                     ```\n\
                     git bisect start\n\
                     git bisect bad HEAD\n\
                     git bisect good v1.2.0\n\
                     # git checkout автоматически между N коммитами\n\
                     # ты тестишь, git bisect good|bad\n\
                     git bisect reset\n\
                     ```\n\
                     С `git bisect run ./test.sh` — fully automated.\n\n\
                     **`git reflog`** — Time machine. Всегда восстанавливай через reflog:\n\
                     - `git reflog` — список всех HEAD movements\n\
                     - `git reset --hard HEAD@{2}` — undo последнюю операцию\n\n\
                     **Hooks** (`.git/hooks/`):\n\
                     - `pre-commit` — lint/format перед commit\n\
                     - `commit-msg` — conventional commits валидация\n\
                     - `pre-push` — run tests перед push\n\
                     - **`pre-commit framework`** (Python) — shared hooks между разработчиками\n\n\
                     **`git worktree`** — несколько checked-out branches одновременно без clone:\n\
                     - `git worktree add ../hotfix hotfix-branch`".into() },
                Snippet { key: "regex".into(), title: "Regex — частые паттерны для логов".into(), body:
                    "**Базовые классы:**\n\
                     - `\\d` digit, `\\w` word char (letter/digit/_), `\\s` whitespace\n\
                     - `[^abc]` — НЕ a/b/c\n\
                     - `\\b` — word boundary (важно для match слов в тексте)\n\n\
                     **Quantifiers:**\n\
                     - `*` 0+, `+` 1+, `?` 0-1, `{n,m}` range\n\
                     - **Lazy:** `*?`, `+?` — match минимум (для `<.*?>`)\n\n\
                     **Useful patterns:**\n\
                     - IP: `\\b(?:\\d{1,3}\\.){3}\\d{1,3}\\b`\n\
                     - Email (rough): `[\\w.+-]+@[\\w-]+\\.[\\w.-]+`\n\
                     - URL: `https?://\\S+`\n\
                     - Hex color: `#[0-9a-fA-F]{6}`\n\
                     - UUID: `[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}`\n\
                     - ISO timestamp: `\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}`\n\n\
                     **Lookarounds (продвинутое):**\n\
                     - `foo(?=bar)` — foo с bar после (не consume)\n\
                     - `foo(?!bar)` — foo БЕЗ bar после\n\
                     - `(?<=bar)foo` — foo с bar перед\n\
                     - `(?<!bar)foo` — foo БЕЗ bar перед\n\n\
                     **Производительность:**\n\
                     - Избегай **catastrophic backtracking**: `(a+)+b` на `aaaaaaaaa!` зависнет на час\n\
                     - Anchor: `^foo` лучше `foo` для matches с начала строки\n\
                     - В Python: `re.compile(...)` для repeated use".into() },
                Snippet { key: "perf-tips".into(), title: "Web app perf — 10 квик-винов".into(), body:
                    "1. **Включи gzip/brotli** на nginx/CDN — 3-5× меньше HTTP payload\n\
                     2. **Cache-Control headers** для статики (`max-age=31536000, immutable` для hashed assets)\n\
                     3. **HTTP/2 или HTTP/3** — мультиплексинг, header compression\n\
                     4. **CDN** для всего static — Cloudflare/Fastly/CloudFront\n\
                     5. **DB connection pool** (PgBouncer / Hikari) — переиспользование TCP+SSL handshakes\n\
                     6. **N+1 → JOIN или batch fetch** — самый частый бекенд-bottleneck\n\
                     7. **Index missing columns** в часто-фильтруемых WHERE/JOIN\n\
                     8. **Eager-load** для known relations (Rails `includes`, Django `select_related`)\n\
                     9. **Lazy-load** images / iframes (`loading=\"lazy\"`)\n\
                     10. **Bundle splitting** на front-end — отдельный bundle per route\n\n\
                     **Метрики которые юзер реально чувствует:**\n\
                     - **LCP** (Largest Contentful Paint) — когда основной контент виден. Цель <2.5s\n\
                     - **FID/INP** — interaction latency. Цель <100ms\n\
                     - **CLS** — layout shift score. Цель <0.1\n\
                     - **TTFB** — time to first byte. Цель <800ms\n\
                     - Real User Monitoring (RUM): web-vitals.js + send to analytics".into() },
                Snippet { key: "interview-tips".into(), title: "Interview tips — как структурировать ответ на behavioral".into(), body:
                    "**STAR framework:**\n\
                     - **Situation** — короткий контекст (1-2 предложения, не саговая ёлка)\n\
                     - **Task** — твоя ответственность в этой ситуации\n\
                     - **Action** — что ИМЕННО ты сделал (\"я\", не \"мы\"). Конкретика\n\
                     - **Result** — измеримый исход + что узнал\n\n\
                     **Анти-паттерны:**\n\
                     - **\"Мы переделали систему\"** — кто конкретно ты? Что делал?\n\
                     - **30-минутная сага** без структуры — интервьюер потеряется\n\
                     - **Только успехи** — спросят failure, не готов = красный флаг\n\
                     - **\"Я попросил у команды помощи\"** как finale — что СДЕЛАЛ потом?\n\n\
                     **Типичные вопросы (заготовь 2-3 истории):**\n\
                     - Tell me about a conflict с коллегой\n\
                     - Project что failed / scope creep\n\
                     - Time you had to learn что-то быстро\n\
                     - Difficult technical decision\n\
                     - Когда не соглашался с менеджером\n\
                     - Самый proud project\n\
                     - Mistake / regret\n\n\
                     **Reverse interview questions (ты к интервьюеру):**\n\
                     - Что для вас был самый интересный technical challenge here last quarter?\n\
                     - Как выглядит typical week для этой роли?\n\
                     - On-call rotation / pager hygiene?\n\
                     - Career path / promotion criteria?\n\
                     - Как принимаются технические решения (RFC? консенсус? CTO декрет?)".into() },
                Snippet { key: "salary".into(), title: "Salary negotiation — как обсуждать".into(), body:
                    "**До интервью:**\n\
                     - **Сам узнай рынок** — Levels.fyi / Glassdoor / habr salaries / индустриальные опросы\n\
                     - Знай свой **walk-away number** (минимум за который пойдёшь) и **target**\n\
                     - Compensation = base + bonus + equity + sign-on + relocation + benefits — не путай!\n\n\
                     **\"Ваши ожидания?\":**\n\
                     - **Никогда не называй первое число** if you can avoid\n\
                     - Try: \"я ищу что в рынке для senior X роли в этом регионе, что вы готовы предложить?\"\n\
                     - Если давят: дай **range, не точку**, и **anchor 10-15% выше target**\n\
                     - Format: \"$210k-$240k base, ожидаю total comp $X с учётом equity\"\n\n\
                     **При offer:**\n\
                     - **Не отвечай сразу** — \"Спасибо, мне нужно подумать, отвечу до X\". Стандартная практика.\n\
                     - **Counter offer письменно** — конкретные числа, fact-based justification (\"конкурент X offer мне Y\")\n\
                     - **Total comp** компонентам — base vs bonus vs equity vs sign-on. Иногда легче сдвинуть один\n\
                     - **Sign-on bonus** — обычно компенсирует unvested equity со старого места\n\n\
                     **Никогда:**\n\
                     - Не врать про competing offers (могут проверить через recruiter network)\n\
                     - Не accept на word — wait for written offer letter\n\
                     - Не сжигать мосты — даже если walking away".into() },
    ]
}

pub fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir().context("no config dir")?;
    let dir = base.join("overlay-mvp");
    std::fs::create_dir_all(&dir).ok();
    Ok(dir.join("config.json"))
}

pub fn load() -> Config {
    let mut cfg = match config_path().and_then(|p| {
        let bytes = std::fs::read(&p).context("read config")?;
        let cfg: Config = serde_json::from_slice(&bytes).context("parse config")?;
        Ok(cfg)
    }) {
        Ok(cfg) => cfg,
        Err(e) => {
            log::warn!("config load failed ({e}), using defaults");
            Config::defaults()
        }
    };
    // Auto-populate newly-added fields when an OLD config file is loaded
    // that pre-dates them. Without this, serde defaults to empty Vec and
    // the user sees "no snippets" with no obvious way to get the canned
    // SRE templates back.
    if cfg.snippets.is_empty() {
        cfg.snippets = Config::defaults().snippets;
        // Persist so the user sees them in Settings AND can edit them on disk.
        let _ = save(&cfg);
        log::info!("auto-populated default snippets into config (was empty)");
    }
    cfg
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path()?;
    let bytes = serde_json::to_vec_pretty(cfg)?;
    std::fs::write(&path, bytes).context("write config")?;
    Ok(())
}

/// Phase E6 v28 — export the full config (INCLUDING ai_bearer +
/// groq_api_key) to an arbitrary path the user picks. Pretty JSON so
/// it's human-editable. The caller is responsible for warning that
/// the file contains secrets.
pub fn export_to(path: &std::path::Path, cfg: &Config) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(cfg).context("serialize config")?;
    std::fs::write(path, bytes).context("write export")?;
    Ok(())
}

/// Phase E6 v28 — import a config from an arbitrary path, validate by
/// deserializing into `Config` (unknown fields ignored, missing fields
/// filled by serde defaults), then persist to the canonical location.
/// Returns the imported Config so the caller can re-apply live state.
pub fn import_from(path: &std::path::Path) -> Result<Config> {
    let bytes = std::fs::read(path).context("read import file")?;
    let cfg: Config = serde_json::from_slice(&bytes).context("parse import JSON")?;
    save(&cfg).context("persist imported config")?;
    Ok(cfg)
}

/// Global, thread-safe handle.
pub type SharedConfig = Arc<RwLock<Config>>;

pub fn shared() -> SharedConfig {
    Arc::new(RwLock::new(load()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write a config to a tmp file, read it back via raw serde_json,
    /// verify all fields match. Doesn't use the global config_path() —
    /// uses an explicit tmpfile to keep tests hermetic.
    #[test]
    fn config_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");

        let mut original = Config::defaults();
        original.meeting_context = "Test SRE interview context".into();
        original.ai_model = "claude-opus-4-7".into();
        original.stealth_enabled = true;
        original.context_profiles = vec![
            ContextProfile {
                name: "k8s".into(),
                context: "kubernetes intro".into(),
            },
            ContextProfile {
                name: "aws".into(),
                context: "aws basics".into(),
            },
        ];
        original.active_profile = Some("k8s".into());

        let bytes = serde_json::to_vec_pretty(&original).unwrap();
        std::fs::write(&path, &bytes).unwrap();

        let raw = std::fs::read(&path).unwrap();
        let loaded: Config = serde_json::from_slice(&raw).unwrap();

        assert_eq!(loaded.meeting_context, original.meeting_context);
        assert_eq!(loaded.ai_model, original.ai_model);
        assert_eq!(loaded.stealth_enabled, original.stealth_enabled);
        assert_eq!(loaded.context_profiles.len(), 2);
        assert_eq!(loaded.context_profiles[1].name, "aws");
        assert_eq!(loaded.active_profile.as_deref(), Some("k8s"));
    }

    /// Old config files won't have new fields (stealth_enabled, prep_model,
    /// trigger_keywords, etc). #[serde(default)] on struct should silently
    /// fill them with defaults instead of failing.
    #[test]
    fn config_partial_json_uses_serde_defaults() {
        // Minimal file — just ai_model. Everything else must come from defaults.
        let minimal = r#"{"ai_model":"claude-old"}"#;
        let cfg: Config = serde_json::from_str(minimal).expect("must parse with defaults");
        assert_eq!(cfg.ai_model, "claude-old");
        // Fields not in JSON default to their Default impl (empty strings, false, None).
        assert_eq!(cfg.ai_bearer, "");
        assert!(!cfg.stealth_enabled);
        assert!(cfg.context_profiles.is_empty());
        assert!(cfg.active_profile.is_none());
    }

    #[test]
    fn config_empty_object_yields_all_defaults() {
        // Even "{}" must parse — every field has a default.
        let cfg: Config = serde_json::from_str("{}").expect("empty object should parse");
        assert_eq!(cfg.ai_bearer, "");
        assert_eq!(cfg.ai_model, "");
        assert_eq!(cfg.response_language, "");
        assert!(!cfg.stealth_enabled);
    }

    /// REGRESSION: new config fields added in v0.0.2+ must have correct
    /// defaults that are user-friendly. If we change the default, this
    /// test catches it — protects against accidental "all-on-by-default"
    /// surprises for upgrading users.
    #[test]
    fn new_v002_field_defaults() {
        let d = Config::defaults();
        // Cost cap default — 0.0 since v0.0.28 means chip is OFF.
        // Old installs (with explicit value in their config.json) keep
        // their value via the per-field serde(default=...) loader.
        assert!(
            d.max_session_cost_usd.abs() < 0.001,
            "max_session_cost_usd default should be 0.0 (chip off), got {}",
            d.max_session_cost_usd
        );
        // detector_skip_mic ON by default — fix for live regression #96
        // (candidate's own voice shouldn't trigger explanation tiles).
        assert!(
            d.detector_skip_mic,
            "detector_skip_mic default should be true (interview use-case)"
        );
        // post_meeting_debrief OFF by default — opt-in per privacy/cost.
        assert!(
            !d.post_meeting_debrief_enabled,
            "post_meeting_debrief_enabled default should be false (opt-in only)"
        );
    }

    /// Old config files (pre-v0.0.2) lack the new fields. Serde must
    /// fill them with proper defaults via per-field #[serde(default="...")]
    /// attributes — these are the source of forward compat. Struct
    /// Default would also work but the per-field form is what gets used
    /// during deserialization, so we assert THAT path specifically.
    ///
    /// v0.0.28: max_session_cost_usd default flipped 1.00 → 0.0 (chip off).
    #[test]
    fn pre_v002_config_gets_correct_field_defaults_via_serde() {
        // Simulate a v0.0.1 config — has all fields up to v0.0.1 but no
        // max_session_cost_usd or detector_skip_mic.
        let pre_v002 = r#"{
            "ai_model": "claude-haiku-4-5",
            "stealth_enabled": false
        }"#;
        let cfg: Config = serde_json::from_str(pre_v002).expect("must parse old config");
        // Field defaults MUST be applied via serde(default=...) on the
        // field itself:
        assert!(
            cfg.max_session_cost_usd.abs() < 0.001,
            "missing field should fall to 0.0 (cap off) — v0.0.28 default"
        );
        assert!(
            cfg.detector_skip_mic,
            "missing field should fall to true (mic skipped), not false"
        );
        assert!(
            !cfg.post_meeting_debrief_enabled,
            "missing field should fall to false (opt-in)"
        );
    }

    /// Config with EXPLICIT positive cost cap should NOT be overridden
    /// to the 0.0 default — user intent to enable the warning is preserved.
    #[test]
    fn explicit_positive_cost_cap_preserved() {
        let with_cap = r#"{ "max_session_cost_usd": 2.50 }"#;
        let cfg: Config = serde_json::from_str(with_cap).expect("must parse");
        assert!(
            (cfg.max_session_cost_usd - 2.50).abs() < 0.001,
            "explicit positive cap must NOT be replaced with 0.0 default"
        );
    }

    /// Config with EXPLICIT 0 for cost cap stays at 0 (was a meaningful
    /// "I disabled the chip" signal before v0.0.28 — still works the same).
    /// v0.0.28: now matches the default. Test kept to lock in the contract.
    #[test]
    fn explicit_zero_cost_cap_preserved() {
        let with_zero = r#"{ "max_session_cost_usd": 0.0 }"#;
        let cfg: Config = serde_json::from_str(with_zero).expect("must parse");
        assert_eq!(cfg.max_session_cost_usd, 0.0, "explicit 0 stays 0");
    }

    /// REGRESSION: the default models MUST be in the pricing table.
    /// If someone updates Config::defaults() to a newer model but forgets
    /// to add it to crate::ai::pricing_per_million, cost reporting falls
    /// back to "safe upper-bound" sonnet pricing — surprise overpay.
    #[test]
    fn defaults_use_models_present_in_pricing_table() {
        use crate::ai::pricing_per_million;
        let d = Config::defaults();
        // Catch a typo by checking each model resolves to a non-fallback price.
        // Fallback (unknown) is sonnet's price; haiku must NOT be that.
        let (haiku_in, _) = pricing_per_million(&d.ai_model);
        assert!(
            haiku_in < 3.0,
            "default ai_model {} hit fallback pricing",
            d.ai_model
        );
        let (prep_in, _) = pricing_per_million(&d.prep_model);
        assert!(
            prep_in <= 15.0,
            "default prep_model {} unreasonably expensive",
            d.prep_model
        );
    }

    /// REGRESSION: trigger_keywords must include the basic terms that
    /// drove every live-test trigger. Empty or stripped-down keywords =
    /// missed questions during an interview.
    #[test]
    fn trigger_keywords_default_includes_core_devops_terms() {
        let kws = Config::defaults().trigger_keywords;
        for required in [
            "kubernetes",
            "etcd",
            "postgres",
            "linux",
            "nginx",
            "prometheus",
        ] {
            assert!(
                kws.contains(required),
                "default trigger_keywords missing core term '{required}'"
            );
        }
    }

    /// Hotkey defaults must be parseable by tauri-plugin-global-shortcut
    /// (smoke-check via the string format — F-keys and CmdOrCtrl combos).
    #[test]
    fn default_hotkeys_have_sensible_format() {
        let d = Config::defaults();
        for hk in [
            &d.hotkey_ask,
            &d.hotkey_screenshot,
            &d.hotkey_toggle_visibility,
            &d.hotkey_pause_audio,
        ] {
            assert!(!hk.is_empty(), "hotkey must not be empty");
            // Either a function key or a modifier+key combo
            assert!(
                hk.starts_with('F') || hk.contains('+') || hk.len() <= 4,
                "hotkey '{hk}' doesn't look like F-key or modifier combo"
            );
        }
    }

    /// REGRESSION: stealth must default OFF. Live test depends on this
    /// (stealth would hide tiles from screen-share, blocking debugging
    /// scenarios with shared screens).
    #[test]
    fn stealth_defaults_off_for_safer_first_run() {
        assert!(!Config::defaults().stealth_enabled);
    }

    /// auto_tiles_enabled must default ON — the whole product purpose is
    /// auto-suggestions on detected questions.
    #[test]
    fn auto_tiles_default_on() {
        assert!(Config::defaults().auto_tiles_enabled);
    }

    /// Malformed JSON config must NOT panic — load() returns defaults.
    /// We can't test load() directly (it touches APPDATA), but we can
    /// verify the error-tolerance contract via from_slice + match.
    #[test]
    fn malformed_json_parse_errors_caught_gracefully() {
        let bad = b"{not valid json";
        let res: Result<Config, _> = serde_json::from_slice(bad);
        assert!(
            res.is_err(),
            "must error on bad JSON (load() recovers via defaults)"
        );
    }

    /// Wrong field type (string instead of bool) must error — caller falls
    /// back to defaults. Prevents silently accepting `"stealth_enabled":"yes"`
    /// as truthy.
    #[test]
    fn wrong_field_type_errors_dont_coerce() {
        let bad = r#"{"stealth_enabled":"yes"}"#;
        let res: Result<Config, _> = serde_json::from_str(bad);
        assert!(res.is_err(), "string-as-bool must reject, not coerce");
    }

    /// REGRESSION: the snippet library is the user's "instant zero-cost
    /// answer" bank. Live-test corpus + this morning's encyclopedia push
    /// has it at 53 snippets. Anyone removing snippets without thinking
    /// twice should hit this floor.
    #[test]
    fn default_snippets_cover_breadth() {
        let d = Config::defaults();
        assert!(
            d.snippets.len() >= 50,
            "snippet library shrank to {} — must stay ≥50",
            d.snippets.len()
        );
        // Domain coverage spot-check — make sure no whole category was
        // accidentally deleted.
        let keys: Vec<&str> = d.snippets.iter().map(|s| s.key.as_str()).collect();
        for domain in [
            "k8s",
            "pg",
            "incident",
            "sli", // originals
            "linux-oom",
            "linux-net",
            "tcp",
            "dns",
            "tls",
            "redis",
            "kafka",
            "oauth2",
            "docker",
            "aws-vpc",
            "prom",
            "trace",
            "saga",
            "mesh",
        ] {
            assert!(
                keys.contains(&domain),
                "default snippets missing /{domain} (domain coverage regression)"
            );
        }
    }

    /// REGRESSION: trigger keyword pool feeds both detector and Whisper
    /// bias. After the encyclopedia push it sits at 250+ unique tokens.
    /// Floor prevents accidental nuke of the list.
    #[test]
    fn default_trigger_keywords_breadth() {
        let kws = Config::defaults().trigger_keywords;
        let count = kws.split_whitespace().count();
        assert!(
            count >= 150,
            "trigger keyword count dropped to {count} — must stay ≥150"
        );
    }

    /// Default snippets must include the SRE essentials and have unique keys
    /// (the expand_snippet command does case-insensitive lookup; duplicates
    /// silently shadow each other).
    #[test]
    fn default_snippets_present_and_keys_unique() {
        let d = Config::defaults();
        let keys: Vec<String> = d.snippets.iter().map(|s| s.key.to_lowercase()).collect();
        assert!(!keys.is_empty(), "must ship default snippets");
        assert!(keys.contains(&"k8s".to_string()), "missing k8s snippet");
        assert!(keys.contains(&"pg".to_string()), "missing pg snippet");
        assert!(
            keys.contains(&"incident".to_string()),
            "missing incident snippet"
        );
        assert!(keys.contains(&"sli".to_string()), "missing sli snippet");
        let mut sorted = keys.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len(), "snippet keys must be unique");
    }

    /// Snippet body should be non-trivial (some content not just whitespace),
    /// and title should be human-readable.
    #[test]
    fn default_snippets_have_content() {
        for s in Config::defaults().snippets {
            assert!(
                !s.title.trim().is_empty(),
                "snippet {} missing title",
                s.key
            );
            assert!(
                s.body.trim().len() >= 50,
                "snippet {} body too short ({} chars)",
                s.key,
                s.body.len()
            );
        }
    }

    /// Snippet round-trip including the new field.
    #[test]
    fn snippet_serialisation_roundtrip() {
        let original = Snippet {
            key: "test".into(),
            title: "Test title".into(),
            body: "**bold** body with newline\n\nand markdown".into(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: Snippet = serde_json::from_str(&json).unwrap();
        assert_eq!(back.key, original.key);
        assert_eq!(back.title, original.title);
        assert_eq!(back.body, original.body);
    }

    /// Context profile shapes round-trip with empty + non-empty.
    #[test]
    fn context_profile_serialisation_roundtrip() {
        let profiles = vec![
            ContextProfile {
                name: "".into(),
                context: "".into(),
            },
            ContextProfile {
                name: "interview".into(),
                context: "long\nmulti-line\ncontext".into(),
            },
        ];
        let json = serde_json::to_string(&profiles).unwrap();
        let back: Vec<ContextProfile> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[1].context, "long\nmulti-line\ncontext");
    }
}
