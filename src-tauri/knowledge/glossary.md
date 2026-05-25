# Glossary v1

Hand-curated technical glossary for AI-assisted SRE/DevOps interviews.
Each entry: heading line `## <key> [— full name]`, body is 1-3 sentence definition with operational notes.
Parser: split on `\n## `, key = first whitespace-separated token of heading (lowercased).

## kubernetes — k8s
Container orchestration platform. Manages deployment, scaling, healing of containerized apps across a cluster of nodes. Originally Google Borg-derived, donated to CNCF in 2015. Declarative API: you describe desired state, controllers reconcile.

## k3s
Lightweight Kubernetes distribution from Rancher. Single binary ~40 MB, packaged sqlite by default instead of etcd. Good for edge / IoT / dev. Production-ready for small clusters.

## kubectl
CLI for Kubernetes API. Reads `~/.kube/config` for cluster + auth. Most-used verbs: `get`, `describe`, `logs`, `exec`, `apply`, `delete`, `port-forward`.

## kubeadm
Tool for bootstrapping a K8s control plane. `kubeadm init` on master, `kubeadm join` on workers. Doesn't handle node provisioning (use Terraform/Ansible for that).

## kubelet
Per-node agent that talks to API server, runs Pods via container runtime (containerd/CRI-O), reports node + Pod status back. Reads PodSpecs from API or static files.

## kube-proxy
Per-node networking component. Maintains iptables/IPVS rules implementing Service abstraction (ClusterIP virtual IP → backend Pods).

## etcd
Distributed key-value store used by Kubernetes for all cluster state. Raft consensus. Latency-sensitive (NVMe recommended). Backup critical — `etcdctl snapshot save`. Watch quotas (default 2 GB), splits on partition.

## pod
Smallest deployable unit in K8s — wraps 1+ containers sharing network namespace + storage volumes. Pod IP is ephemeral, dies on restart.

## deployment
K8s controller managing replicated stateless Pods. Provides rolling updates with `maxSurge`/`maxUnavailable`, rollback via revision history.

## statefulset
K8s controller for stateful workloads. Stable network identity (`pod-0`, `pod-1`), persistent volume per Pod, ordered start/stop. Use for databases, queues.

## daemonset
K8s controller that runs one Pod per node (or per labeled subset). For node agents: log shippers, monitoring exporters, CNI plugins.

## job
K8s controller that runs Pods to completion. Use for batch tasks. `parallelism` + `completions` config the work pattern.

## cronjob
K8s controller that creates Jobs on a cron schedule. Watch `concurrencyPolicy: Forbid` for long-running tasks.

## service
K8s abstraction giving stable virtual IP + DNS name for a set of Pods (selected by label). Types: ClusterIP (default, internal), NodePort, LoadBalancer, ExternalName.

## ingress
K8s API object describing HTTP/HTTPS routing rules (host/path → service). Needs an Ingress Controller (nginx-ingress, Traefik, Contour, Istio Gateway) to actually implement.

## configmap
K8s object holding non-secret config as key-value pairs. Mounted as files or env vars into Pods. Updates don't auto-restart Pods (need rollout).

## secret
K8s object for sensitive data (passwords, tokens, certs). Base64-encoded (NOT encrypted) by default. Enable EncryptionConfiguration for at-rest encryption.

## pvc — PersistentVolumeClaim
Pod's request for storage (size + access mode). Matched to a PV (PersistentVolume) by the cluster.

## pv — PersistentVolume
Cluster-level storage resource. Provisioned statically by admin or dynamically by StorageClass.

## storageclass
Defines how PVs are dynamically provisioned. Specifies provisioner plugin (csi-driver), parameters (disk type, replication), reclaim policy.

## namespace
Logical partition in a K8s cluster. Default ones: `default`, `kube-system`, `kube-public`. Use for multi-tenancy + RBAC scoping.

## rbac — Role-Based Access Control
K8s auth model. Role/ClusterRole = what's allowed. RoleBinding/ClusterRoleBinding = who gets the role. ServiceAccount = identity for Pods.

## serviceaccount
K8s identity for Pods to call API server. Each Pod gets one (`default` SA in namespace unless overridden). Token mounted at `/var/run/secrets/kubernetes.io/serviceaccount/`.

## helm
Package manager for Kubernetes. A "Chart" is a templated set of YAML manifests + values.yaml. `helm install/upgrade/rollback`.

## kustomize
Native K8s tool for managing YAML overlays. `kustomization.yaml` describes patches per environment. No templating (vs Helm) — pure YAML composition.

## argocd
GitOps CD tool for K8s. Watches Git repo for manifest changes, syncs to cluster. UI for diff/drift detection, sync-waves for ordered deploys.

## flux — Flux CD
GitOps tool, CNCF graduated. More modular than Argo CD (Source + Kustomize + Helm controllers). Better for multi-cluster, multi-tenancy.

## istio
Service mesh: sidecar Envoy proxies handle traffic management, mTLS, observability. Heavy but feature-rich. Components: istiod (control plane), envoy (data plane).

## linkerd
Lightweight service mesh. Rust-based proxy (smaller/faster than Envoy). Simpler than Istio, fewer features.

## envoy
High-performance L7 proxy. Powers Istio, used standalone, basis for Ambassador/Contour. Hot-reloadable config, rich observability.

## cilium
CNI plugin using eBPF for networking + observability + policy. Replaces iptables, much faster at scale. Includes Hubble for service map.

## calico
CNI plugin. Uses BGP for routing (each node = BGP peer). Implements NetworkPolicy. Mature, widely deployed.

## flannel
Simple CNI plugin. VXLAN overlay network. Easy setup, lacks NetworkPolicy support (combine with Calico for that).

## crd — CustomResourceDefinition
K8s extension mechanism. Define new API objects beyond core (Pod/Service/etc). Used by operators (Prometheus Operator's `Prometheus` resource).

## operator
Custom controller managing CRD lifecycle. Encodes domain knowledge (how to deploy + upgrade + recover Postgres, Kafka, etc).

## hpa — HorizontalPodAutoscaler
Scales replicas by metric (CPU/memory/custom). Requires metrics-server installed. Stabilization window: scale-up fast, scale-down slow.

## vpa — VerticalPodAutoscaler
Recommends or sets Pod's CPU/memory requests/limits based on usage. Don't combine with HPA on same metric (they fight).

## keda
Kubernetes Event-Driven Autoscaling. Scales workloads based on event source (Kafka lag, SQS queue depth, Prometheus query). Scale-to-zero capable.

## cluster-autoscaler
Adds/removes nodes when Pods can't be scheduled (pending) or nodes underused. Talks to cloud provider API (AWS ASG, GCP MIG).

## taint
Node attribute preventing Pod scheduling unless Pod has matching toleration. Use for dedicated nodes (GPU, spot instances).

## toleration
Pod attribute allowing scheduling on tainted nodes. Three effects: NoSchedule, PreferNoSchedule, NoExecute (evicts existing).

## affinity
Pod scheduling preference (nodeAffinity, podAffinity, podAntiAffinity). Soft (preferred) or hard (required). Spread workloads across zones with antiAffinity.

## resource-requests
Minimum CPU/memory a Pod needs. Scheduler reserves this on the node. Affects QoS class assignment.

## resource-limits
Max CPU/memory a Pod can use. Exceeding memory limit → OOMKilled. CPU is throttled, not killed.

## qos-class
K8s assigns Pods QoS based on requests/limits: Guaranteed (req=limit), Burstable (req<limit), BestEffort (no req/limit). Evicted in reverse order on node pressure.

## readinessprobe
Per-container check. If failing, Pod removed from Service endpoints (no traffic). Use for slow-starting apps that aren't ready immediately.

## livenessprobe
Per-container check. If failing, container is killed and restarted. Use sparingly — aggressive probes cause cascading restarts.

## startupprobe
Disables livenessProbe until passing once. For very slow-starting apps (large JVMs, complex initialization).

## initcontainers
Containers that run sequentially BEFORE main containers start. Use for setup (chmod data dirs, fetch configs, wait for dependencies).

## sidecar
Co-located container in same Pod as main app. Common: log forwarder, service mesh proxy, config reloader.

## ephemeralcontainer
Temporary container injected into running Pod for debug. `kubectl debug -it pod/X --image=busybox`. K8s 1.25+ stable.

## network-policy
K8s firewall rules at Pod level. Default allow-all unless enforced by CNI (Calico, Cilium). Specify ingress/egress by Pod selector or IP block.

## podsecuritypolicy
Deprecated (removed 1.25). Replaced by Pod Security Standards (Privileged, Baseline, Restricted) enforced via namespace labels.

## admissioncontroller
API server plugin that intercepts requests after auth, before persistence. Validating (yes/no) or mutating (modify object). Webhooks for custom logic.

## opa — Open Policy Agent
General-purpose policy engine. Rego language. Used as K8s admission controller (Gatekeeper) for org-wide policies.

## docker
Container runtime + tooling. CLI, daemon, image format. Originally introduced container concept to mainstream. In K8s, replaced as runtime by containerd (k8s 1.20+).

## containerd
CRI-compliant container runtime. Spun out of Docker. Used by K8s, more lightweight than full docker daemon.

## cri — Container Runtime Interface
K8s API for container runtimes. Implementations: containerd, CRI-O, dockerd-CRI-shim (deprecated).

## cri-o
Container runtime built specifically for K8s CRI. Lighter than containerd, used by RedHat OpenShift.

## runc
Low-level OCI-compliant container runtime. Actually starts containers (namespaces + cgroups). Used by containerd, CRI-O.

## oci — Open Container Initiative
Standards body for container image format + runtime spec. Docker/containerd/CRI-O all implement OCI specs.

## podman
Docker-compatible CLI without daemon. Rootless containers by default. Pod concept (groups of containers sharing namespace). Red Hat sponsored.

## buildah
Tool for building OCI images without daemon. Lower-level than docker build. Often paired with Podman.

## skopeo
Tool to copy/inspect/sign OCI images between registries. No daemon needed.

## distroless
Container base images with no shell, no package manager — only the app + minimal libs. Google project. Reduces attack surface + size.

## scratch
Empty Docker base image (FROM scratch). Used for static binaries (Go, Rust). Smallest possible image.

## multistage-build
Dockerfile pattern: builder stage compiles, final stage copies binary. Reduces image size 10-100×.

## dockerfile
Text file with image build instructions. Each instruction = layer. Order matters: stable layers up, frequently-changing down (cache friendly).

## docker-compose
YAML file describing multi-container apps. `docker compose up` starts services. Good for local dev, not for production K8s scale.

## buildkit
Modern Docker build backend. Supports cache mounts, secrets, multi-platform, parallel stages. Default since Docker 23+.

## image-layer
Read-only filesystem snapshot in an image. Stacked via overlayfs at runtime. Each `RUN`/`COPY`/`ADD` adds a layer.

## image-digest
SHA256 hash of the image manifest. Pin by digest (`image:tag@sha256:abc...`) for immutability in production.

## image-pull-policy
K8s Pod spec: `Always`, `IfNotPresent` (default for tagged), `Never`. Use `Always` for `:latest` (else stale).

## image-registry
Server hosting container images. Docker Hub (public), ECR/GCR/ACR (cloud), Harbor (self-hosted), GitHub Container Registry.

## image-scanner
Tool checking images for known CVEs. Trivy (popular, free), Grype, Snyk, Clair, Anchore. Integrate in CI before push.

## init — PID 1
Process started first in container. Must reap zombies + handle signals. Use tini, dumb-init, or `--init` flag. Without it, SIGTERM may not propagate.

## entrypoint
Dockerfile/Pod field: the program to run. CMD provides default args. Override CMD at runtime, not ENTRYPOINT (usually).

## healthcheck — Docker HEALTHCHECK
Container-level health probe. Less powerful than K8s probes. Set status reportable via `docker ps`.

## linux
Open-source Unix-like OS kernel (1991, Torvalds). "Linux" colloquially = kernel + GNU userspace + distro. Powers most servers, Android, embedded.

## kernel
Core of OS. Manages CPU, memory, devices, syscalls. Linux is monolithic but supports loadable modules. `/proc/kallsyms` exposes symbols, `uname -r` shows version.

## syscall
User-space → kernel API. Examples: `read`, `write`, `open`, `fork`, `execve`, `mmap`, `epoll_wait`. `strace -p PID` traces. ~400 on Linux x86_64.

## procfs — /proc
Pseudo-filesystem exposing kernel + process state. `/proc/<pid>/maps` (memory), `/proc/cpuinfo`, `/proc/meminfo`, `/proc/net/tcp`.

## sysfs — /sys
Pseudo-filesystem exposing kernel objects/devices. Hardware info, module params, cgroup files (legacy v1).

## cgroups — control groups
Kernel feature for resource limiting per process group. Containers use cgroups for CPU/memory/IO limits. v1 (legacy) vs v2 (unified hierarchy, current default).

## namespaces
Kernel feature isolating resources per process. Types: PID, NET, MNT, UTS, IPC, USER, CGROUP, TIME. Containers = namespaces + cgroups + chroot.

## systemd
Init system + service manager on most modern Linux. Replaces SysV init. Units: services, sockets, timers, mounts. `systemctl`, `journalctl`.

## journald
systemd's structured logging service. Stores binary journals (`/var/log/journal/`). Query: `journalctl -u nginx -f`, `journalctl -p err -b`.

## logrotate
Rotates + compresses log files. Config in `/etc/logrotate.d/`. Often called via cron or systemd timer.

## bash
GNU shell. Most popular Linux shell. Features: globbing, brace expansion, arrays, `[[ ]]` tests. Avoid for serious scripts (use Python/Go).

## zsh
Z Shell. Superset of bash with better completion, themes (oh-my-zsh). Default on modern macOS.

## fish — friendly interactive shell
Autosuggestions, syntax highlighting, no POSIX compat. Sometimes Sysadmin daily driver, scripts portable elsewhere.

## ssh
Secure Shell — encrypted remote login + tunnel. Default port 22. Config: `~/.ssh/config` aliases, `authorized_keys` for pubkey auth.

## scp
Secure copy over SSH. `scp file user@host:/path`. Newer alternative: `rsync -avz file user@host:/path` (resumable, faster, diffs).

## rsync
Sync files locally or over SSH. Efficient (only sends diffs). Flags: `-a` archive, `-v` verbose, `-z` compress, `--delete` remove extras.

## strace
Trace syscalls of a process. `strace -p PID` attach, `strace -f -o log.txt cmd` follow forks. Slows target ~10×.

## ltrace
Trace library calls (vs syscalls). Useful for dynamic linking issues, malloc patterns.

## lsof — list open files
`lsof -p PID` files held by process, `lsof -i :443` who listens on port, `lsof | grep deleted` deleted-but-held files (disk leak source).

## ss — socket statistics
Faster netstat. `ss -tnp` TCP listeners with PIDs, `ss -tn state established` active connections, `ss -s` summary.

## netstat
Old socket stats tool. Mostly replaced by `ss`. Still common in scripts.

## tcpdump
Packet capture CLI. `tcpdump -i any -nn host X.X.X.X port 443 -w out.pcap`. Wireshark for GUI analysis.

## wireshark
Packet analyzer with GUI. Opens .pcap files. Filter syntax different from tcpdump's BPF.

## nc — netcat
"TCP/UDP Swiss army knife". `nc -vz host port` test connectivity, `nc -l 12345` listen, port-forward, file transfer.

## curl
HTTP client CLI. `curl -v -H 'X: Y' -d '@body.json' https://api`. Flags: `-I` HEAD only, `-L` follow redirects, `-o file` save.

## wget
File downloader. `wget -c URL` resume, `wget -m URL` mirror site. Often pre-installed where curl isn't.

## jq
JSON processor CLI. `curl ... | jq '.items[].name'`. Essential for working with API responses.

## yq
YAML processor analogous to jq. Variants exist (kislyuk/yq, mikefarah/yq) with slightly different syntax.

## grep
Pattern search. `grep -r 'pattern' dir` recurse, `grep -E 'a|b' file` extended regex, `grep -v` invert match, `grep -c` count.

## awk
Field processor + small language. `awk '{print $1, $3}' file`, `awk -F, '$3 > 100' csv`. POSIX standard, gawk has extensions.

## sed — stream editor
In-place edit / replace. `sed -i 's/old/new/g' file`, `sed -n '10,20p' file`. POSIX vs GNU sed flag differences.

## find
File search. `find . -name '*.log' -mtime -7 -delete` delete logs newer than 7d. `find / -type f -size +1G` find big files.

## xargs
Build commands from stdin. `find ... | xargs rm`, but use `-print0 | xargs -0` for spaces in names. `-P 4` parallelism.

## parallel — GNU parallel
Run jobs concurrently from list. `parallel -j 8 ./process {} ::: file1 file2 file3`. Better than xargs for complex pipelines.

## tmux
Terminal multiplexer. Persists sessions across SSH disconnects. Keybindings: `Ctrl-b d` detach, `Ctrl-b %` vsplit. Config `~/.tmux.conf`.

## screen
Older terminal multiplexer. `screen -R name` reattach. Mostly replaced by tmux.

## vim
Modal editor. Insert/Normal/Visual modes. Steep learning curve, powerful once learned. Plugin systems: vim-plug, Vundle.

## nvim — Neovim
Modern vim fork. Lua config, async plugins (LSP, treesitter). Better extensibility.

## htop
Interactive process viewer. CPU/memory per process, kill via F9, sort by column.

## top
Classic process viewer. Press `M` sort memory, `P` sort CPU, `1` per-CPU breakdown.

## ps
Snapshot of processes. `ps aux` BSD format all users, `ps -ef` long. Pipe to grep often.

## kill
Send signal to process. `kill -9 PID` SIGKILL (no cleanup), `kill -15 PID` SIGTERM (default, graceful), `kill -HUP PID` reload.

## signals — Unix signals
SIGHUP (reload), SIGINT (Ctrl-C), SIGTERM (graceful stop), SIGKILL (force), SIGUSR1/2 (app-defined), SIGCHLD (child died), SIGPIPE (broken pipe).

## nohup
Run command immune to hangups. `nohup ./long-task &` lets you logout without killing. Modern alternative: `systemd-run --user`.

## disown
Detach job from shell. `./task &` then `disown`. Useful if forgot nohup.

## crontab
Schedule jobs. `crontab -e` edit. Format: `m h dom mon dow cmd`. `*/5` every 5 min. Test with `* * * * *` (every minute).

## at
One-off scheduled job. `echo 'cmd' | at now + 2 hours`. Requires atd service.

## df
Disk free. `df -h` human-readable, `df -i` inodes. Inode exhaustion separate from byte exhaustion (small files).

## du
Disk usage per directory. `du -hx --max-depth=1 /var | sort -h`. `-x` stays on one filesystem.

## ncdu
Interactive du. TUI for navigating + deleting. Faster than du for exploration.

## free
Memory stats. `free -h`. Note: `available` is what apps can use; `free` is misleadingly low due to cache.

## vmstat
Virtual memory / IO / CPU stats. `vmstat 1 10` 10 samples 1s apart. `si/so` are swap in/out (should be 0 in healthy system).

## iostat
Disk IO stats. `iostat -xz 1` extended every second. Watch `%util` (saturation), `await` (latency).

## sar
System Activity Reporter. Historical stats from sysstat package. `sar -u 1` CPU, `sar -r 1` memory, `sar -n DEV 1` network.

## dmesg
Kernel ring buffer. Boot messages, OOM kills, hardware errors. `dmesg -T` human time, `dmesg -wH` follow.

## journalctl
systemd journal query. `-u unit`, `-p priority`, `--since '1 hour ago'`, `-f` follow. `--vacuum-size=100M` shrink.

## iptables
Userspace tool for netfilter rules. Tables (filter, nat, mangle), chains (INPUT, OUTPUT, FORWARD, PREROUTING). Replaced by nftables on modern systems.

## nftables
Replacement for iptables/ip6tables/arptables/ebtables. Unified syntax, atomic rule updates. `nft list ruleset`.

## ufw — Uncomplicated Firewall
Friendly frontend for iptables. `ufw allow 22`, `ufw enable`. Ubuntu default.

## firewalld
Dynamic firewall daemon for RHEL/CentOS. Zone-based. `firewall-cmd --add-port=80/tcp --permanent`.

## selinux
Mandatory access control (NSA-origin). Process + file labels. Often disabled because of complexity. `getenforce`, `setenforce 0`, `audit2allow`.

## apparmor
MAC alternative to SELinux. Profile per binary path. Used by Ubuntu, snaps, Docker.

## ulimit
Per-process resource limits. `ulimit -n` open files (default 1024 often too low), `ulimit -u` processes. Persisted via `/etc/security/limits.conf`.

## sysctl
Kernel parameters runtime config. `sysctl -a` list all, `sysctl -w net.core.somaxconn=65535` set. Persist in `/etc/sysctl.conf`.

## modprobe
Load kernel module. `modprobe nf_conntrack` for connection tracking. `lsmod` list loaded, `rmmod` unload.

## udev
Device manager. Rules in `/etc/udev/rules.d/`. Names persistent devices (e.g. eth0 → enp0s3 via predictable network names).

## lvm — Logical Volume Manager
Disk abstraction. Physical Volume (PV) → Volume Group (VG) → Logical Volume (LV). Resize without unmount, snapshots.

## raid
Redundant Arrays of Independent Disks. Levels: 0 (stripe, fast no redundancy), 1 (mirror), 5 (parity, 1 fail OK), 6 (2 fails OK), 10 (mirror+stripe).

## mdadm
Linux software RAID tool. `mdadm --create /dev/md0 --level=10 --raid-devices=4 /dev/sd[bcde]`.

## zfs
Combined volume manager + filesystem. Copy-on-write, snapshots, send/receive, integrity checksums. Originally Solaris, now OpenZFS.

## btrfs
Copy-on-write filesystem with snapshots, send/receive, RAID. Less mature than ZFS for RAID5/6. Default for openSUSE, Fedora /home.

## ext4
Default Linux filesystem for years. Journaled. Stable, well-understood. Max file 16 TiB, FS 1 EiB.

## xfs
High-performance journaled FS. RHEL default. Better for large files + parallel IO than ext4.

## inode
Filesystem metadata block for a file (permissions, timestamps, size, block pointers). Filesystem can run out of inodes separately from bytes.

## mount
Attach filesystem. `mount /dev/sdb1 /mnt`, `umount /mnt`. Persist via `/etc/fstab`.

## fstab
`/etc/fstab` — boot-time mounts. Fields: device, mountpoint, fstype, options, dump, fsck-order.

## chmod
Change file permissions. Octal: `chmod 755 file` (rwx-rx-rx). Symbolic: `chmod u+x file`. `+s` setuid.

## chown
Change file owner. `chown user:group file`. `-R` recursive (dangerous on /, use carefully).

## sudo
Run command as another user (usually root). `/etc/sudoers` config (edit via `visudo`). `sudo -i` interactive shell as root.

## su
Switch user. `su -` becomes root with their environment. `su username` keeps your env.

## ps1
Bash prompt string. `\u@\h \w \$` user@host workdir $. Customize for git status: parse `git rev-parse` in `$(...)`.

## bashrc
`~/.bashrc` — sourced for interactive non-login shells. Aliases, functions, prompt.

## bash-profile
`~/.bash_profile` — sourced for login shells. Often just sources .bashrc. PATH adjustments here.

## history
Command history. `Ctrl-R` reverse search. `HISTSIZE=10000` in bashrc. `!!` last command, `!n` command N.

## alias
Shell abbreviation. `alias k=kubectl`. Persist in .bashrc/.zshrc. Doesn't take args (use function for that).

## function
Bash function: `myfunc() { echo "$1"; }`. Local vars: `local x=...`. Use `set -euo pipefail` at script top.

## exit-code
Last command's status. `0` success, non-zero failure. `$?` access. `set -e` exits script on first non-zero.

## pipe — |
Connect stdout of one command to stdin of next. `cmd1 | cmd2`. `set -o pipefail` makes pipeline exit on any stage's failure.

## redirect
`>` overwrite, `>>` append, `<` input, `2>` stderr, `&>` both, `2>&1` stderr-to-stdout. `>/dev/null` discard.

## heredoc
Inline multi-line input. `cmd <<EOF ... EOF`. `<<-EOF` strips leading tabs. `<<'EOF'` no variable expansion.

## glob
Shell wildcards: `*` any chars, `?` one char, `[abc]` set, `[!abc]` negation, `**` recursive (with shopt -s globstar).

## env
Show env vars. Also: command prefix `env VAR=val cmd`. Common vars: PATH, HOME, USER, LANG, LD_LIBRARY_PATH.

## path-variable
`$PATH` — colon-separated dirs searched for commands. `which cmd` shows resolved location.

## ldd
Show shared libraries needed by binary. `ldd /usr/bin/ls`. Linker resolution: `LD_LIBRARY_PATH`, `/etc/ld.so.conf`.

## file
Identify file type by magic bytes. `file binary` → "ELF 64-bit LSB executable". Useful for unknown files.

## readelf
Read ELF binary headers + sections. `readelf -d binary` dynamic deps, `readelf -a` everything.

## objdump
Disassemble + inspect object files. `objdump -d binary` disassembly. Useful for security/reverse-eng.

## hexdump
Hex view of binary. `hexdump -C file | head`. `xxd` similar.

## md5sum
MD5 hash. Use sha256sum for integrity (MD5 broken for security). `md5sum -c sums.txt` verify.

## sha256sum
SHA-256 hash. Generate: `sha256sum file > sum`. Verify: `sha256sum -c sum`.

## gpg
GnuPG — encryption + signing. `gpg --import key`, `gpg --verify file.sig`, `gpg --encrypt --recipient X file`.

## openssl
Crypto toolkit. `openssl s_client -connect host:443` TLS debug, `openssl x509 -in cert.pem -text` parse cert.

## openrc
Init system used by Alpine. Lighter than systemd.

## sysv-init
Original Unix init. Scripts in `/etc/init.d/`. Largely replaced by systemd.

## ld — dynamic linker
`/lib/ld-linux-x86-64.so.2` — runtime linker for ELF binaries. Resolves shared libs at exec.

## ld-conf
`/etc/ld.so.conf` + `/etc/ld.so.conf.d/*.conf` — system shared lib paths. Run `ldconfig` after edit.

## ssh-agent
Holds decrypted private keys in memory. Avoid re-typing passphrase. `ssh-add ~/.ssh/id_ed25519`.

## ssh-key
`~/.ssh/id_ed25519` (preferred over RSA). Pub: `id_ed25519.pub`. `ssh-keygen -t ed25519 -C 'comment'` generate.

## known-hosts
`~/.ssh/known_hosts` — server fingerprints accepted. `ssh-keygen -R host` remove stale.

## authorized-keys
`~/.ssh/authorized_keys` — pubkeys allowed to login as this user. Permissions must be 600.

## proxyjump
SSH option: connect through bastion. `ssh -J bastion target` or `~/.ssh/config: ProxyJump bastion`.

## scp-vs-sftp
SCP is older, simpler. SFTP supports more (resume, listing) via SSH subsystem. Modern OpenSSH uses SFTP backend for SCP by default.

## locale
Language/region config. `LANG=en_US.UTF-8`, `LC_ALL` overrides all. `locale -a` list available.

## timezone
`/etc/localtime` symlink to zoneinfo. `timedatectl set-timezone Europe/Moscow`. NTP sync via chrony or systemd-timesyncd.

## chrony
NTP client/server. Faster sync than ntpd, better for laptops. `chronyc tracking` status, `chronyc sources` peers.

## cron-vs-systemd-timer
Cron simpler. systemd timers more flexible (Calendar events, persistent across boot, journal logging).

## tini
PID-1 init for containers. Reaps zombies, forwards signals. Use as Docker `ENTRYPOINT` to avoid orphaned processes.

## dumb-init
Alternative to tini. Same purpose: proper signal handling + zombie reaping in containers.

## containerd-shim
Long-running process keeping a container alive after containerd restart. Per-container.

## overlayfs
Union FS used by container runtimes. Stacks read-only image layers with a writable upper layer. Default Docker storage driver.

## tar
Archive tool. `tar czf out.tgz dir/` create gzipped, `tar xzf in.tgz` extract. `-v` verbose.

## gzip
DEFLATE compression. `gzip file` → file.gz. `pigz` is parallel version (8× faster on multi-core).

## zstd
Modern compression by Facebook. Better ratio + speed than gzip. Default in many distros.

## xz
LZMA compression. Best ratio, slower. `tar Jcf` xz, `tar Jxf` decompress.

## tar-vs-zip
tar groups files, doesn't compress alone. zip combines both. Unix prefers tar+gzip (separated concerns).

## ext-attr — extended attributes
File metadata beyond standard (owner, perms). SELinux labels stored here. `getfattr`/`setfattr`.

## acl — file ACLs
Fine-grained permissions beyond owner/group/other. `getfacl file`, `setfacl -m u:user:rw file`.

## proc-stat
`/proc/<pid>/stat` — process status (state, parent, CPU, memory). Used by ps/top.

## proc-status
`/proc/<pid>/status` — human-readable version of /proc/PID/stat. Shows VmRSS, VmPeak, threads.

## proc-fd
`/proc/<pid>/fd/` — symlinks to file descriptors. Useful: `ls -l /proc/PID/fd | wc -l` count open files.

## proc-net
`/proc/net/` — network stats. `/proc/net/tcp` connections, `/proc/net/netstat` extended.

## proc-sys
`/proc/sys/` — kernel parameters. Same as sysctl interface.

## kdump
Kernel crash dumps. Reserves memory at boot for capture kernel. Used for post-mortem.

## perf
Performance analysis tool. `perf top` system-wide profiling, `perf record -p PID`, `perf report`. Linux kernel project.

## ftrace
Kernel function tracing. `/sys/kernel/debug/tracing/`. `trace-cmd` wrapper.

## bpf — Berkeley Packet Filter
In-kernel VM originally for packet filtering. Now general-purpose (eBPF).

## ebpf — extended BPF
Programmable kernel hooks for observability/networking/security. `bcc-tools`, `bpftrace`, Cilium, Falco all use eBPF.

## bpftrace
DTrace-like CLI for eBPF. `bpftrace -e 'kprobe:vfs_read { @[comm] = count(); }'` count reads per program.

## bcc-tools
Suite of eBPF tracing tools. `execsnoop`, `opensnoop`, `tcpconnect`, `biolatency`. Easy starting point.

## flamegraph
Visualization of profiler output. Wide = much time, deep = call depth. `perf record` → `flamegraph.pl` (Brendan Gregg).

## stress-ng
Stress testing tool. `stress-ng --cpu 4 --vm 2 --vm-bytes 1G` simulate load. For chaos testing, capacity planning.

## fio
Flexible IO tester. Measure disk throughput/IOPS/latency. Critical before relying on cloud disk specs.

## iperf3
Network throughput tool. Server side `iperf3 -s`, client `iperf3 -c host`. TCP + UDP modes.

## mtr — my traceroute
Combination of ping + traceroute, continuously updating. Shows packet loss + latency per hop.

## traceroute
Show routing path to host. `-T` TCP (bypasses ICMP blocks), `-p 443` specific port.

## dig
DNS lookup. `dig +short example.com`, `dig +trace` follow root → TLD → authoritative, `dig @8.8.8.8` specific resolver.

## nslookup
Older DNS lookup tool. Use dig instead — better output, more features.

## host
DNS lookup CLI. `host example.com`. Simpler output than dig.

## resolved — systemd-resolved
Default DNS resolver on modern Linux. Caches, handles split DNS. `resolvectl status` shows config.

## nss — name service switch
`/etc/nsswitch.conf` — how to resolve users, groups, hosts. `hosts: files dns` means /etc/hosts first.

## resolv-conf
`/etc/resolv.conf` — DNS servers. On systemd-resolved systems usually symlink.

## ip — iproute2
Modern replacement for ifconfig/route. `ip addr show`, `ip route get IP`, `ip link set eth0 up`. Use ip not ifconfig.

## ifconfig
Old network tool. `ifconfig` deprecated; use `ip` from iproute2 package. Still common in legacy scripts.

## brctl
Linux bridge management. `brctl show`, `brctl addif br0 eth0`. Replaced by `ip link` newer.

## ovs — Open vSwitch
Programmable Layer-2 switch. Software-defined networking. Used by OpenStack, K8s CNIs.

## vlan
Layer-2 isolation tag (802.1Q). Multiple virtual networks on one physical link. `ip link add link eth0 name eth0.10 type vlan id 10`.

## bond
Link aggregation. Multiple NICs as one. Modes: round-robin, active-backup, 802.3ad LACP. `/etc/network/interfaces` or systemd-networkd.

## vxlan
Virtual eXtensible LAN. Layer-2 overlay over Layer-3. Used by Flannel CNI, Docker overlay network.

## wireguard
Modern VPN. Simpler + faster than OpenVPN/IPsec. Kernel module + `wg` CLI. Use for site-to-site or roadwarrior.

## openvpn
Mature VPN over TLS. Slower than WireGuard but more featureful (LDAP, MFA integrations).

## ipsec
Site-to-site VPN protocol. Strongswan/Libreswan on Linux. More complex than WireGuard.

## haproxy
TCP/HTTP load balancer + proxy. Mature, high-performance. ACL-based routing, stats page, hot reload.

## nginx
HTTP server + reverse proxy + LB. Async + event-loop. Configs in `/etc/nginx/`. Hot reload: `nginx -s reload`.

## apache — httpd
Original web server. Module-rich (mod_php, mod_ssl). Process/thread worker models. Heavier than nginx.

## caddy
HTTP server with automatic HTTPS (Let's Encrypt). Simple Caddyfile. Good for quick deploys.

## envoy-l7-proxy
L7 proxy by Lyft. Powers Istio. Dynamic config via xDS API. Rich observability (stats, tracing).

## traefik
Cloud-native reverse proxy. Auto service discovery (K8s, Docker, Consul). Automatic HTTPS.

## consul
Service discovery + KV store + health checking + service mesh (Connect). HashiCorp.

## etcd-vs-consul
etcd Raft-based, K8s state store. Consul broader (service discovery, multi-DC, ACLs). Both KV stores.

## zookeeper
Coordination service. Original of the genre (HBase, Kafka, ZK-using stuff). Replaced in newer Kafka by KRaft.

## vault — HashiCorp Vault
Secrets management. Dynamic secrets (DB creds on demand), transit encryption, PKI. Audit log.

## terraform
Infrastructure as Code. HCL syntax. State in S3 + DynamoDB lock. `plan` → `apply`. Modules for reuse.

## ansible
Agentless config management. YAML playbooks over SSH. Idempotent modules. No master required.

## puppet
Pull-based config management. Ruby DSL. Master + agents. Pre-Ansible era dominant.

## chef
Config management with Ruby DSL. "Cookbooks" + "recipes". Agent-based (chef-client). Acquired by Progress.

## saltstack
Config management + remote execution. ZeroMQ transport, salt-minion agents. Faster than Ansible for large fleets.

## packer
Image builder for cloud (AMI, GCP image, Vagrant box). HashiCorp. Templates in HCL/JSON.

## vagrant
VM provisioning for local dev. Box-based (pre-built images). Replaced by docker-compose mostly.

## cloud-init
Multi-platform VM initialization. User-data script run on first boot. Used by AWS, GCP, OpenStack.

## kvm — Kernel-based VM
Linux hypervisor. Type-1 in practice. Used by libvirt (virsh), Proxmox, OpenStack.

## qemu
Machine emulator + virtualizer. Often paired with KVM (qemu-kvm). Slow without HW accel.

## libvirt
Hypervisor abstraction. `virsh` CLI manages KVM/QEMU/Xen. Used by RHEL virtualization stack.

## proxmox
Open-source virtualization platform. Web UI on top of KVM + LXC. Cluster + HA features.

## vmware
Proprietary virtualization. ESXi hypervisor, vCenter management. Acquired by Broadcom 2023.

## xcp-ng
Open-source XenServer fork. Type-1 hypervisor. Less common than KVM-based stacks.

## openstack
Open-source IaaS platform. Modules: Nova (compute), Neutron (network), Cinder (storage), Keystone (auth). Heavy, complex.

## lxc
Linux Containers. OS-level virtualization predating Docker. Heavier (full init), used by Proxmox for app containers.

## lxd
Container manager built on LXC. Acts more like a hypervisor for containers. Canonical project.

## qemu-img
Disk image tool. `qemu-img create -f qcow2 disk.qcow2 20G`. Convert between formats: raw, qcow2, vmdk.

## qcow2
QEMU Copy-On-Write 2 disk format. Sparse, snapshots, compression. Standard for KVM.

## ovmf — OVMF UEFI firmware
Open-source UEFI for QEMU/KVM. Required for Secure Boot, large disk support.

## tcp — Transmission Control Protocol
Reliable, ordered, stream-oriented L4 protocol. 3-way handshake (SYN/SYN-ACK/ACK). Congestion control (Reno, CUBIC, BBR). Default for most apps.

## udp — User Datagram Protocol
Connectionless, unreliable L4 protocol. No handshake, no ordering. Used for DNS, RTP, QUIC base.

## quic
UDP-based transport with built-in TLS + multiplexing. Foundation for HTTP/3. Lower handshake latency than TCP+TLS.

## ip-protocol — Internet Protocol
L3 packet routing. v4 (32-bit, mostly exhausted) and v6 (128-bit, plentiful). Best-effort, unreliable.

## icmp
Network diagnostic protocol. ping (echo request/reply), traceroute (TTL exceeded), unreachable messages.

## arp — Address Resolution Protocol
Maps IPv4 → MAC on local segment. `arp -n` or `ip neigh show`. Replaced by NDP in IPv6.

## ndp — Neighbor Discovery Protocol
IPv6 equivalent of ARP. Uses ICMPv6.

## dhcp — Dynamic Host Configuration Protocol
Auto-assigns IP + gateway + DNS to clients. Lease-based. `dhclient` Linux client.

## dns — Domain Name System
Distributed name → IP resolution. UDP/53 default, TCP/53 for big responses, TLS/853 (DoT), HTTPS/443 (DoH).

## bgp — Border Gateway Protocol
Routing between autonomous systems (the Internet). Policy-based. Calico CNI uses iBGP for cluster routing.

## ospf — Open Shortest Path First
Interior gateway routing protocol. Link-state, fast convergence. Common in enterprise.

## nat — Network Address Translation
Rewrites src/dst IPs. SNAT (outbound, masquerade), DNAT (port forward). On NAT GW: ephemeral port exhaustion is a real concern.

## pat — Port Address Translation
NAT variant: multiple internal IPs → one external IP, distinguished by port. Most home routers.

## vpn — Virtual Private Network
Encrypted tunnel over public network. Site-to-site or roadwarrior. Modern: WireGuard, IPsec, OpenVPN.

## mpls — MultiProtocol Label Switching
Labels packets for fast forwarding. Used by ISPs for traffic engineering, VPNs.

## sdn — Software-Defined Networking
Decoupled control + data plane. OpenFlow, Open vSwitch. Programmable network behavior.

## mtu — Maximum Transmission Unit
Largest packet without fragmentation. 1500 standard Ethernet. Jumbo frames 9000 in data center. Wrong MTU → black-hole on tunnels.

## mss — Maximum Segment Size
TCP payload size = MTU - 40 (IP+TCP headers). MSS clamping rewrites SYN to prevent fragmentation downstream.

## ttl — Time To Live
IP packet field. Decremented per hop. Reaches 0 → drop + ICMP "time exceeded" (used by traceroute).

## socket
OS endpoint for network communication. (IP, port, protocol). Stream socket = TCP, datagram = UDP.

## port
16-bit number on IP. 0-1023 privileged (need root), 1024-49151 registered, 49152-65535 ephemeral.

## ephemeral-port
Client-side dynamic port. ~28k per IP. Exhaustion causes "cannot assign requested address". Tune via `net.ipv4.ip_local_port_range`.

## connection-tracking
Stateful firewall feature. Kernel module `nf_conntrack` tracks established connections. Table can overflow under DDoS.

## keepalive — TCP keepalive
Periodic empty packets to detect dead connections. Off by default for OS, can be enabled via sockopt. Different from application keepalive (HTTP/2 PING).

## conntrack
Utility for connection tracking inspection. `conntrack -L` list connections, `conntrack -E` events stream.

## syn-flood
DDoS: many SYN, no ACK. Backlog fills. Mitigations: `net.ipv4.tcp_syncookies`, larger backlog.

## reset — TCP RST
Abnormal connection termination. Sent on unexpected packet (no socket listening, FIN out of order).

## time-wait
TCP state after connection close (2× MSL, ~60s). Prevents stale packets from new connection. Many = short-lived connections, consider keepalive.

## close-wait
TCP state where remote sent FIN but local hasn't close()'d. Bug in app code if persistent. Many = file descriptor leak.

## established
Working TCP connection state. `ss -tn state established | wc -l` counts open connections.

## listen
TCP state where server bound + listening on port. Backlog queue holds pending accept().

## somaxconn
`net.core.somaxconn` — kernel max backlog queue. Default 4096 (5.4+), bump to 65535 for high-traffic.

## tcp-tw-reuse
Reuse TIME_WAIT sockets for new outbound connections. Set to 1 if you make many outbound short connections.

## bbr — TCP BBR congestion control
Google's congestion algorithm. Throughput model based on bandwidth × latency. Often dramatically faster than CUBIC on lossy links.

## cubic
Default Linux TCP congestion control. Cubic function over time. Better than Reno for high-bandwidth links.

## reno — TCP Reno
Classic congestion control (slow-start, fast retransmit, fast recovery). Foundation for newer algorithms.

## ecn — Explicit Congestion Notification
Routers mark packets to signal congestion BEFORE drop. Both ends must support. Off by default in Linux.

## qos — Quality of Service
Traffic prioritization. Linux `tc` (traffic control), DSCP markings. Often disabled in cloud.

## tls — Transport Layer Security
Encryption protocol. v1.3 modern (1-RTT, faster), v1.2 still common, older versions insecure. Replaces SSL.

## ssl
Predecessor to TLS. SSL 2.0/3.0 broken. "SSL certificate" terminology persists; technically TLS now.

## https — HTTP over TLS
HTTP encrypted with TLS. Default port 443. Modern web requires HTTPS (HSTS, CSP, mixed-content blocks).

## http2
Major HTTP revision. Multiplexing, header compression (HPACK), server push (rarely used), binary framing. Requires TLS in practice (browsers).

## http3
HTTP over QUIC (UDP-based). 0-RTT resumption, no head-of-line blocking, faster mobile networks.

## tls-handshake
Negotiates ciphersuite, exchanges keys, validates cert. TLS 1.3 = 1 RTT, TLS 1.2 = 2 RTT.

## sni — Server Name Indication
TLS extension sending hostname in ClientHello. Lets one IP serve multiple TLS sites. Standard since 2010s.

## alpn — Application-Layer Protocol Negotiation
TLS extension negotiating app protocol (HTTP/1.1, HTTP/2, HTTP/3). Avoids extra round-trip.

## cipher-suite
TLS spec for: key exchange + auth + bulk encryption + MAC. Modern suites: TLS_AES_256_GCM_SHA384.

## hsts — HTTP Strict Transport Security
Header telling browser to always use HTTPS for domain. Defeats SSL-strip attacks. Preload list available.

## csp — Content Security Policy
Header restricting script/image sources. Defense vs XSS. Granular: `script-src 'self' https://cdn`.

## cors — Cross-Origin Resource Sharing
Browser-enforced rules for cross-origin requests. Server sends `Access-Control-Allow-Origin` headers.

## cdn — Content Delivery Network
Globally-distributed cache. Cloudflare, Fastly, CloudFront, Akamai. Reduces origin load, lowers user latency.

## cache-control
HTTP header. `max-age=N`, `s-maxage` (shared cache), `no-store` (don't cache), `immutable` (versioned assets).

## etag
HTTP entity tag header. Hash of response. Client sends `If-None-Match`, server returns 304 if match. Saves bandwidth.

## webhook
HTTP callback. Server pushes event to consumer URL. Signed via HMAC for auth.

## websocket
Full-duplex TCP-like over HTTP/1.1 upgrade. ws:// or wss://. Heartbeat (ping/pong) to detect disconnect.

## grpc
HTTP/2-based RPC. ProtoBuf encoding. Streaming bidirectional. Strong typing via .proto files.

## protobuf — Protocol Buffers
Google's IDL + binary encoding. Schema-evolving (field numbers). Smaller + faster than JSON.

## rest — REST API
Architectural style. Resources via URLs, verbs are HTTP methods. Stateless. JSON typical body.

## graphql
API query language. Client requests specific fields. Single endpoint. Avoids over/under-fetching. Subscription model for real-time.

## soap
XML-based RPC. Heavy, schema-driven (WSDL). Legacy enterprise. Mostly replaced by REST/gRPC.

## json — JavaScript Object Notation
Text data format. Universal interop. Verbose but human-readable. Use jq for CLI processing.

## yaml — YAML Ain't Markup Language
Superset of JSON. Whitespace-sensitive. K8s manifests, CI configs. Watch tabs vs spaces (use spaces).

## toml — Tom's Obvious Minimal Language
Config format. Used by Rust Cargo, Python pyproject.toml. More readable than YAML for flat configs.

## xml — Extensible Markup Language
Verbose markup language. SOAP, RSS, SVG. Largely replaced by JSON in APIs. Still used in enterprise (Java, Microsoft).

## csv — Comma-Separated Values
Tabular plaintext. Watch for: embedded commas (need quoting), Windows CRLF vs Unix LF, encoding (UTF-8 BOM).

## avro
Apache binary serialization with schema. Kafka common. Better than JSON for big data (smaller, schema evolution).

## parquet
Columnar binary format. Big data analytics. Compresses well, reads fast for column-subset queries. Spark/Athena native.

## msgpack
Binary JSON-like format. Compact. Used by Redis (RESP2 extension), some game servers.

## load-balancer-l4
Layer 4 LB (TCP/UDP). Fast, no protocol awareness. AWS NLB, HAProxy mode TCP, IPVS.

## load-balancer-l7
Layer 7 LB (HTTP). Routing by host/path/header. AWS ALB, nginx, Envoy. Slower but smarter.

## round-robin
LB algorithm: rotate through backends. Simple, ignores load.

## least-connections
LB algorithm: send to backend with fewest active connections. Better for long-lived (websocket, DB).

## ip-hash
LB algorithm: hash(client-IP) → backend. Sticky sessions. Breaks if client behind NAT.

## consistent-hashing
Hash ring distributing keys across nodes. Add/remove node moves only 1/N keys. Used by caches, distributed DBs.

## sticky-session
LB pinning a client to a backend. Cookie or IP-hash based. Avoid if possible (state-in-pod = no sticky needed).

## health-check
LB endpoint probe. Failed → remove from rotation. Interval 5-10s, threshold 2-3 fails. Different from K8s liveness/readiness.

## reverse-proxy
Server forwarding to backend. nginx/HAProxy/Envoy. Adds: TLS termination, caching, rate-limiting, A/B routing.

## forward-proxy
Server on behalf of clients (Squid). Use cases: corporate egress, anonymization, content filtering.

## socks5
Generic proxy protocol. Routes TCP/UDP. SSH supports as `-D` dynamic forwarding (poor man's VPN).

## haproxy-stats
HAProxy stats page (port 9000 typical). Per-backend health, request rates, errors. Auth-protect in prod.

## anycast
Same IP advertised from multiple locations via BGP. Routers pick nearest. Used by Google DNS (8.8.8.8), CDNs.

## unicast
Standard: one IP, one destination. Default for TCP/UDP.

## multicast
One sender, many subscribers. IGMP/PIM. Used in datacenter for service discovery (mDNS), media streaming.

## broadcast
Same as multicast but to all on segment. ARP, DHCP DISCOVER. Not routable across networks.

## ftp
File Transfer Protocol. Active vs passive mode. Cleartext (FTPS adds TLS, SFTP is SSH-based).

## smtp
Simple Mail Transfer Protocol. Server-to-server mail. Port 25 (default), 587 (submission with auth).

## imap
Mailbox-on-server email protocol. Keeps mail synced. Port 993 (with TLS).

## pop3
Download-and-delete email. Port 995 (with TLS). Mostly superseded by IMAP.

## ntp — Network Time Protocol
Time synchronization. Critical for: TLS cert validity, log correlation, distributed systems consensus.

## ldap — Lightweight Directory Access Protocol
User/group directory. AD on Windows, OpenLDAP/389-ds on Linux. Often used for SSO backend.

## kerberos
Ticket-based auth. Used by AD, can be backbone for SSO. Time-sensitive (NTP critical).

## radius
AAA protocol (authentication/authorization/accounting). Used by VPNs, WiFi enterprise, network gear.

## saml — Security Assertion Markup Language
XML-based SSO standard. Browser-redirected. Slower than OIDC, common in enterprise.

## oauth2
Authorization framework. Access tokens, scopes, flows. NOT auth (use OIDC). RFC 6749.

## oidc — OpenID Connect
OAuth 2.0 + identity layer. Returns ID token (JWT) with user claims. Use for "Login with Google/Microsoft".

## jwt — JSON Web Token
Compact signed token. Header.Payload.Signature, base64url-encoded. Often used as bearer in HTTP `Authorization`.

## jwks
JSON Web Key Set. `/.well-known/jwks.json` endpoint with public keys. Used to verify JWT signature.

## mtls — Mutual TLS
Both client and server present certs. Used in service-to-service auth (service mesh).

## pki — Public Key Infrastructure
Cert hierarchy: Root CA → Intermediate → Leaf. Trust anchor in OS/browser cert stores.

## ca — Certificate Authority
Issues certs. Root CAs trusted by OS. Public CAs (Let's Encrypt, DigiCert). Private CA for internal services.

## acme
Automated cert issuance protocol. Used by Let's Encrypt. `certbot`, `acme.sh`, `cert-manager` in K8s.

## csr — Certificate Signing Request
What you send to CA. Contains pubkey + identity info. `openssl req -new -key ...`.

## x509
Cert format standard. PEM (base64) or DER (binary). `openssl x509 -in cert.pem -text`.

## crl — Certificate Revocation List
List of revoked certs. Browser checks before trusting. Slow, mostly replaced by OCSP.

## ocsp
Real-time cert revocation check. OCSP Stapling: server fetches + caches response.

## hsts-preload
List of domains hardcoded into browsers as HTTPS-only. Submit at hstspreload.org.

## sse — Server-Sent Events
HTTP streaming from server to client (text/event-stream). Simpler than WebSocket for one-way data.

## chunked-encoding
HTTP/1.1 transfer encoding for unknown-length response. Each chunk has size prefix.

## etag-weak
Prefix `W/`. Semantic equivalence (e.g. compressed vs uncompressed). Strong etag requires byte-identical.

## referer
HTTP header with previous URL. Misspelled in spec. Privacy controls via Referrer-Policy.

## user-agent
HTTP header identifying client. Often spoofed. Used for browser detection (avoid—use feature detection).

## accept
HTTP header listing acceptable content types. `Accept: application/json` for APIs.

## content-type
HTTP header declaring body's media type. `application/json; charset=utf-8`.

## status-code
HTTP response code. 1xx info, 2xx success, 3xx redirect, 4xx client error, 5xx server error.

## status-100
Continue. Client should send body after this (rare, used with Expect: 100-continue).

## status-101
Switching Protocols. Used for WebSocket upgrade.

## status-200
OK. Default success.

## status-201
Created. POST that created a resource. Should include Location header.

## status-204
No Content. Success but no body (e.g. DELETE).

## status-301
Moved Permanently. Cache forever, update bookmarks. SEO-friendly.

## status-302
Found / Temporary Redirect. Don't cache.

## status-304
Not Modified. Cache validation succeeded (If-None-Match matched ETag).

## status-307
Temporary Redirect. Like 302 but preserves HTTP method.

## status-308
Permanent Redirect. Like 301 but preserves HTTP method.

## status-400
Bad Request. Client-side error in syntax.

## status-401
Unauthorized. Missing or invalid auth. Server should send WWW-Authenticate.

## status-403
Forbidden. Auth ok, but no permission.

## status-404
Not Found.

## status-405
Method Not Allowed. Wrong HTTP verb for endpoint.

## status-409
Conflict. Common for optimistic concurrency (ETag mismatch on PUT).

## status-410
Gone. Resource permanently removed. Use sparingly (404 is fine usually).

## status-413
Payload Too Large. Body exceeds server limit.

## status-422
Unprocessable Entity. Syntax OK but semantics wrong. Common for validation errors.

## status-429
Too Many Requests. Rate limit. Server should send Retry-After.

## status-500
Internal Server Error. Generic server failure. Don't include stack trace in body.

## status-502
Bad Gateway. Proxy couldn't reach upstream.

## status-503
Service Unavailable. Temporary, often during deploy. Send Retry-After.

## status-504
Gateway Timeout. Proxy waited too long for upstream.

## rate-limit
Restriction on request rate. Common headers: X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset.

## token-bucket
Rate limit algorithm. Bucket holds N tokens, refills at rate R, each request takes 1. Allows bursts.

## leaky-bucket
Rate limit: fixed output rate, requests queue. Smooths bursts, drops on overflow.

## sliding-window
Rate limit: count requests in last N seconds (sliding). More memory than fixed-window, accurate at boundary.

## fixed-window
Rate limit: count per discrete window (per minute). Simple, but allows 2× burst at boundary.

## back-pressure
Downstream signals upstream to slow down. Critical for stream processing. Implemented via: blocking queues, gRPC flow control, reactive streams.

## circuit-breaker
Pattern: open circuit on N consecutive failures, fail fast, periodically half-open to test. Libraries: resilience4j, polly.

## retry-with-backoff
On failure, wait + retry. Exponential: 1s, 2s, 4s, 8s. Add jitter (random multiplier) to avoid thundering herd.

## jitter
Random component in backoff to spread retries. Full jitter: `delay = random(0, base * 2^attempt)`.

## idempotent
Same operation N times = same result. GET, PUT, DELETE should be idempotent. POST often isn't (use Idempotency-Key header).

## idempotency-key
Header value uniquely identifying request. Server dedupes retries. Stripe, PayPal use this pattern.

## etag-optimistic-concurrency
Client sends If-Match: <etag> on PUT. Server checks current etag, returns 412 if mismatch. Avoids lost updates.

## eventual-consistency
After updates stop, all replicas converge. DynamoDB, S3 (historical), Cassandra defaults. Read-your-writes is stronger.

## strong-consistency
All reads see most recent write. Single-leader replication usually. Linearizable = strong.

## linearizable
Strongest consistency. Operations appear instantaneous, in some order. Hardest to scale.

## serializable
Transactions appear as if executed serially. SQL TX isolation level. Costs latency vs lower levels.

## cap-theorem
Consistency, Availability, Partition tolerance — pick 2. In practice, partition tolerance is mandatory, so trade C vs A.

## acid
Atomicity, Consistency, Isolation, Durability. Classical DB transaction properties. Relational DBs.

## base
Basically Available, Soft state, Eventually consistent. NoSQL alternative model to ACID.

## paxos
Consensus algorithm. Hard to implement. Multi-Paxos for stream of values.

## raft
Consensus algorithm. Simpler than Paxos. Used by etcd, Consul, CockroachDB. Leader election + log replication.

## leader-election
Distributed protocol picking one node as leader. Raft, Bully algorithm, ZooKeeper leases.

## split-brain
Network partition causing two leaders. Disaster: divergent state. Mitigation: quorum (majority votes).

## quorum
Minimum nodes for valid decision. Usually `floor(N/2) + 1`. Raft, Paxos use quorums.

## fencing
Preventing old leader from causing damage after failover. STONITH (Shoot The Other Node In The Head), lease tokens.

## sharding
Data partitioning across nodes. Horizontal scale. Shard key choice critical (hot spots, resharding pain).

## consistent-hashing-vs-modulo
Modulo: change N moves N-1/N keys. Consistent hash: change moves 1/N. Critical for caches, sharded DBs.

## replication-master-slave
One write node, many read replicas. Async (fast, may lose recent writes on failover) or sync (consistent, slower).

## replication-multi-master
All nodes accept writes. Conflict resolution needed (last-write-wins, CRDT, app-level).

## replication-leader-follower
Modern term for master-slave. PostgreSQL streaming replication, MySQL replication.

## wal — Write-Ahead Log
Every change logged before applying. Crash recovery: replay WAL. Postgres pg_wal, MySQL binlog, RocksDB.

## binlog — MySQL binary log
WAL equivalent. Replication source for replicas. Statement / row / mixed format.

## redo-log
InnoDB's WAL. ib_logfile0/1. Crash recovery on restart.

## undo-log
InnoDB. Stores old values for MVCC + rollback.

## mvcc — Multi-Version Concurrency Control
Readers don't block writers. Each transaction sees consistent snapshot. PostgreSQL + MySQL InnoDB use it.

## isolation-level-read-uncommitted
Lowest SQL isolation. Reads see uncommitted changes (dirty reads). Rarely useful.

## isolation-level-read-committed
Default in Postgres. No dirty reads. Non-repeatable reads + phantom reads still possible.

## isolation-level-repeatable-read
Default in MySQL InnoDB. Snapshot at start of TX. No non-repeatable reads. Phantoms can occur in standard SQL but InnoDB prevents.

## isolation-level-serializable
Highest. Equivalent to serial execution. Locks or serialization errors. Use for critical financial logic.

## phantom-read
SQL anomaly. Re-running same query in TX returns new rows (inserted by other TX).

## non-repeatable-read
SQL anomaly. Same row queried twice gives different values (other TX updated).

## dirty-read
Reading uncommitted changes from another TX. Only at READ UNCOMMITTED.

## two-phase-commit
Distributed TX protocol. Prepare → commit. Coordinator SPOF. Slow. Saga preferred for microservices.

## saga
Long-running TX as sequence of local TXs with compensating actions. No global lock. See snippet /saga.

## cap-vs-pacelc
PACELC extends CAP: when Partitioned, Availability vs Consistency. Else, Latency vs Consistency. Captures normal-mode tradeoffs.

## crdt — Conflict-free Replicated Data Type
Distributed data structures merging without coordination. Counters, sets, registers. Used by Redis CRDTs, Riak.

## vector-clock
Causality tracking in distributed systems. Each node increments own counter on each event. Comparison: < > = || (concurrent).

## lamport-timestamp
Logical clock. Send: increment. Receive: max(local, received) + 1. Establishes happens-before relation.

## happens-before
Partial order on events: a → b if a causes b. Used in concurrency reasoning (Java Memory Model, distributed systems).

## postgres — PostgreSQL
Mature open-source RDBMS. MVCC, rich SQL, extensions (PostGIS, TimescaleDB). Default for new web apps.

## pgbouncer
Lightweight PG connection pooler. Modes: session, transaction (most common), statement.

## pgpool
PG pooler + load balancer + replication coordinator. More features than PgBouncer, more complex.

## patroni
HA framework for PostgreSQL. Uses etcd/Consul/ZK for leader election. Auto-failover. De facto standard.

## repmgr
Older PG HA tool. Manual switchover, no auto-failover unless paired with pgwatch/keepalived.

## walg — WAL-G
PG backup tool. Continuous WAL archiving to S3 + base backups + PITR.

## pgbackrest
Enterprise-grade PG backup. Incremental, parallel, encryption. More features than wal-g.

## pg-stat-statements
PG extension tracking query stats (calls, total_time, rows). Top of "what's slow" investigation.

## explain
PG/MySQL query plan visualization. `EXPLAIN ANALYZE` runs the query + shows timing. BUFFERS for cache hits.

## explain-analyze
Actually executes query. Watch out: side effects (UPDATEs) happen. Wrap in BEGIN; EXPLAIN ANALYZE ...; ROLLBACK.

## autovacuum
PG background process reclaiming dead rows + updating statistics. Critical — tune for write-heavy workloads.

## vacuum
PG command marking dead tuples reusable. VACUUM FULL rewrites table (locks, slow). Autovacuum usually enough.

## bloat — table bloat
Wasted space from dead rows. `pgstattuple` extension measures. Long transactions block vacuum.

## hot-tuple
PG "Heap-Only Tuple" — UPDATE fits in same page, no index update. Optimization, doesn't apply if any indexed column changed.

## index-bloat
B-tree pages with deletes. REINDEX CONCURRENTLY rebuilds without lock.

## reindex
Rebuild PG index. CONCURRENTLY for online operation.

## partition — PG table partitioning
Split table by range/list/hash. Improves query performance (partition pruning) + maintenance (drop old partitions vs DELETE).

## logical-replication
PG10+ table-level replication. CREATE PUBLICATION + CREATE SUBSCRIPTION. Cross-version capable. No DDL replication.

## streaming-replication
PG physical replication via WAL. Whole-cluster. `pg_basebackup -R` initial copy.

## hot-standby
PG replica accepting read queries. `hot_standby = on` default.

## synchronous-commit
PG durability setting. `on` (default) waits for WAL flush. `off` faster but risks data loss on crash.

## synchronous-standby
PG sync replication. Wait for replica WAL ack before commit. Survives primary loss but slows writes.

## work-mem
PG per-operation memory (sort, hash). Multiplied by concurrent ops × operations per query. Tune carefully.

## shared-buffers
PG cache. ~25% RAM typical. Larger isn't always better (OS already caches).

## effective-cache-size
PG planner hint. Total cache (shared_buffers + OS). ~75% RAM. Affects plan choice.

## random-page-cost
PG planner: cost of random page fetch. Default 4.0 assumes spinning disk. Set 1.1 for NVMe SSDs.

## jit — PostgreSQL JIT
LLVM-based compilation for hot queries. Win for analytical workloads. Adds overhead for OLTP — `jit=off` can help.

## pgvector
PG extension for vector similarity search. Embeddings for RAG, recommendation. IVFFlat / HNSW indexes.

## postgis
PG extension for geographic data. ST_* functions for spatial queries. Industry standard for GIS.

## timescaledb
PG extension for time-series. Auto-partitioning by time. Compression. Continuous aggregates.

## mysql
Open-source RDBMS. Acquired by Oracle. InnoDB default engine. Heavily used (WordPress, MediaWiki).

## mariadb
Fork of MySQL by original creator. Drop-in compatible mostly. Open governance via foundation.

## innodb
MySQL default engine. ACID, MVCC, foreign keys, crash recovery. Clustered index on PK.

## myisam
Old MySQL engine. No transactions, table-level locking. Avoid for new use.

## buffer-pool
InnoDB cache. `innodb_buffer_pool_size = 70-80% RAM`. Critical for performance.

## innodb-flush-log-at-trx-commit
1 = sync flush each commit (durable, slow). 2 = flush each commit, sync each second (compromise).

## query-cache
MySQL feature (removed in 8.0). Cached identical query results. Was a scalability bottleneck.

## innodb-io-capacity
MySQL background IO operations per second. 200 default (spinning), 2000+ for SSD.

## gtid — Global Transaction ID
MySQL replication ID surviving topology changes. Simplifies failover.

## binlog-format-row
MySQL binlog records row changes (not statements). Safer for replication, larger logs.

## semisync
MySQL semi-synchronous replication. Wait for at least one replica's ack. Data durability without full sync cost.

## sql-mode
MySQL behavior modes. STRICT_TRANS_TABLES rejects invalid data instead of truncating. ONLY_FULL_GROUP_BY enforces SQL standard.

## redis
In-memory key-value store. Optional persistence. Used as cache, queue, pub/sub, lock.

## redis-sentinel
Redis HA solution. Monitors master/replicas, automatic failover. Apps connect to Sentinel for routing.

## redis-cluster
Sharded Redis. 16384 hash slots distributed. Multi-key ops only within hash tag `{user:1}:foo`.

## rdb — Redis RDB
Snapshot persistence. Periodic full state to disk. Fast restart, may lose recent writes.

## aof — Redis AOF
Append-Only File. Each write logged. `appendfsync everysec` standard. Combined with RDB for prod.

## redis-lua
Atomic scripting. Lua script executes as single op. Use for complex multi-key atomic logic.

## redis-streams
Persistent log structure (5.0+). Consumer groups, ack, replay. Like Kafka-lite.

## redis-pubsub
Fire-and-forget broadcast. No persistence. Subscribers must be connected.

## memcached
Distributed in-memory cache. Simpler than Redis (no persistence, fewer datatypes). Multi-threaded.

## mongo — MongoDB
Document database (BSON). Flexible schema. Replica sets, sharding. Aggregation pipeline.

## mongo-replica-set
Min 3 nodes (PSS or PSA). Primary handles writes, secondaries replicate. Auto-failover via election.

## mongo-shard-key
Critical choice. Immutable, partitions data. Hashed (uniform) or ranged (range queries fast).

## mongo-aggregation
Pipeline of stages: $match, $group, $project, $lookup. SQL-like operations in document model.

## mongo-index
B-tree (default), compound (multi-field, ESR order), hashed, text, geospatial, partial.

## cassandra
Wide-column distributed DB. AP system (eventual consistency tunable). Linear scale, no SPOF.

## clickhouse
Columnar OLAP DB. Excellent for analytics over billions of rows. Used by Cloudflare, Uber, Datadog.

## merge-tree
ClickHouse default table engine family. Background merges of inserted parts.

## cockroachdb
Distributed SQL. Postgres wire-compatible. Multi-region writes via Raft. Strong consistency at scale.

## yugabytedb
Distributed SQL (PG-compatible). Raft per shard. Similar niche to CockroachDB.

## tidb
Distributed SQL (MySQL-compatible). TiKV storage layer (Raft). Hybrid OLTP/OLAP.

## elasticsearch
Distributed search engine (Lucene-based). Full-text + analytics. JSON API. ELK stack core.

## opensearch
AWS fork of Elasticsearch (after license change). Apache 2.0. Compatible.

## kibana
Visualization for Elasticsearch. Dashboards, log exploration.

## opensearch-dashboards
OpenSearch's Kibana fork.

## fluentbit
Lightweight log shipper (C). K8s native. Forwards to Elasticsearch, Loki, S3, etc.

## fluentd
Older log shipper (Ruby). More plugins than Fluent Bit but heavier.

## logstash
Heavyweight log processor (JVM). Rich filtering/transforms. Often replaced by lighter shippers.

## vector
Log/metric router by Datadog (Rust). Fast, flexible transforms (VRL language).

## kafka
Distributed log. Topics with partitions. Consumer groups. Persistent. Used for event streaming.

## kafka-connect
Framework for streaming data in/out of Kafka. Source + Sink connectors for DBs, S3, ES.

## kafka-streams
Java library for stream processing. Stateful operations (joins, windows). Backed by Kafka topics for state.

## ksqldb
SQL over Kafka topics. CREATE STREAM, CREATE TABLE. Backed by Kafka Streams.

## kafka-mirror-maker
Cross-cluster Kafka replication. v2 uses Kafka Connect.

## kraft — Kafka KRaft
Kafka's replacement for ZooKeeper. Built-in Raft consensus. Stable in Kafka 3.3+.

## consumer-offset
Per-consumer-group position in partition. Committed to `__consumer_offsets` topic. Determines replay.

## kafka-partition
Ordered log within topic. Parallelism unit. # partitions ≥ # consumers in group.

## kafka-replication-factor
Copies of each partition. 3 typical. `min.insync.replicas` how many must ack.

## kafka-acks
Producer setting. `0` fire-and-forget, `1` leader ack, `all` (or `-1`) all in-sync replicas.

## rabbitmq
Message broker. Multiple protocols (AMQP, MQTT, STOMP). Flexible routing via exchanges.

## amqp
Advanced Message Queuing Protocol. RabbitMQ default. Connection → channel → queue/exchange.

## exchange-direct
RabbitMQ: routing key == binding key. Exact match.

## exchange-topic
RabbitMQ: wildcards in routing key. `logs.*.error`, `logs.#`.

## exchange-fanout
RabbitMQ: broadcast to all bound queues.

## quorum-queue
RabbitMQ HA queue. Raft consensus across nodes. Replaces classic mirrored queues.

## nats
Lightweight messaging. Subjects + pub/sub. JetStream for persistence.

## nats-jetstream
Persistent + replay layer on NATS. Streams + consumers. Lighter than Kafka.

## pulsar
Apache distributed messaging. Tiered storage (BookKeeper). Multi-tenancy. Geo-replication built-in.

## activemq
JMS-compatible broker. Older. Replaced in many places by RabbitMQ/Kafka.

## sqs
AWS Simple Queue Service. Standard (at-least-once) or FIFO. Visibility timeout.

## sns
AWS Simple Notification Service. Pub/sub. Targets: SQS, Lambda, HTTP, email, SMS.

## kinesis
AWS Kafka-like stream. Shards. KCL (consumer lib).

## eventbridge
AWS event bus. SaaS partner events, custom buses, routing rules. Successor to CloudWatch Events.

## prometheus
Pull-based metrics monitoring + alerting. PromQL query language. CNCF graduated.

## alertmanager
Prometheus alert router. Grouping, inhibition, silencing, notification routes.

## prometheus-operator
K8s operator for managing Prometheus instances + ServiceMonitor + PodMonitor CRDs.

## thanos
Long-term Prometheus storage on object store. Global query view across clusters.

## cortex
Multi-tenant Prometheus-as-a-service. Backbone of Grafana Cloud Metrics historically.

## mimir
Grafana Labs' fork of Cortex. Better multi-tenancy, sharding.

## victoriametrics
Prometheus alternative. Better resource efficiency. Compatible PromQL.

## grafana
Visualization. Dashboards from any datasource (Prometheus, Loki, ES, SQL). Alerting v9+ unified.

## loki
Log aggregation by Grafana. Indexes only labels (not content) — dirt cheap storage. LogQL syntax.

## tempo
Grafana's tracing backend. S3-backed. Trace ID lookup focus. Pairs with Loki + Mimir.

## jaeger
Tracing system. Originally Uber. UI for trace exploration. CNCF graduated.

## zipkin
Original distributed tracing system (Twitter). Replaced by Jaeger in many places.

## opentelemetry — otel
Unified observability standard (traces + metrics + logs). SDKs + Collector. Successor to OpenTracing + OpenCensus.

## otel-collector
Vendor-neutral observability data pipeline. Receive → process → export. Use to decouple apps from backend.

## promql
Prometheus query language. `rate()`, `histogram_quantile()`, aggregation (sum, avg by). Multi-dimensional.

## logql
Loki query language. `{job="api"} |= "error"`. Stream selectors + filters.

## traceql
Tempo's query language for traces.

## metric-cardinality
Count of unique label combinations. High cardinality kills Prometheus. Avoid user_id as label.

## datadog
Commercial observability platform. Metrics, traces, logs, APM, RUM, synthetics in one place.

## newrelic
Commercial APM. Code-level transactions, error tracking.

## splunk
Commercial log indexer/search. Powerful query language. Expensive at scale.

## sentry
Error tracking SaaS. Source maps, release tracking, breadcrumbs. Open-source self-hostable.

## bugsnag
Error tracking (Sentry competitor). Acquired by SmartBear.

## rollbar
Error tracking. Smaller than Sentry.

## pagerduty
Incident management + on-call rotation. Schedules, escalation, integrations.

## opsgenie
PagerDuty competitor. Acquired by Atlassian.

## victorops
Splunk's on-call product.

## statuspage
Public-facing status pages. Atlassian product.

## sli — Service Level Indicator
Quantitative measure of service. Availability %, latency p99. See snippet /sli.

## slo — Service Level Objective
Target for SLI. "p99 latency < 200ms". Engineering team's commitment.

## sla — Service Level Agreement
Contract with customer. Includes penalties. Usually weaker than internal SLO.

## error-budget
Allowed unavailability per period. SLO 99.9% → 43min/month. See snippet /errorbudget.

## burn-rate
Speed of error budget consumption. Multi-window alerts (slow burn over hours + fast burn over minutes).

## sli-availability
% of successful requests over total. (200-399) / all. Clear, user-meaningful.

## sli-latency
p50/p95/p99 response time. Distribution matters more than mean (skewed by outliers).

## sli-throughput
Requests/sec. Capacity indicator. Combined with latency for full picture.

## sli-correctness
% of correct results (for search/ML/business logic). Hardest to measure, often via labels.

## red-method
Rate, Errors, Duration. Service-level monitoring (per endpoint).

## use-method
Utilization, Saturation, Errors. Resource-level monitoring (CPU/memory/disk).

## golden-signals
Google SRE book. Latency, traffic, errors, saturation. Per service.

## chaos-engineering
Intentional failure injection in prod. Validate resilience. Tools: Chaos Monkey, Gremlin, Litmus, Chaos Mesh.

## game-day
Scheduled chaos exercise. Practice incident response without real incident.

## postmortem
Post-incident analysis. Blameless culture. Action items with owner + due date.

## blameless
Postmortem culture: focus on systems/processes, not individuals. Encourages honest reporting.

## five-whys
Root cause analysis technique. Ask "why" 5 times to dig past symptoms.

## blast-radius
Scope of failure. Small blast = isolated impact. Architectural goal: shrink blast radius.

## bulkhead
Pattern: isolate components so failure doesn't cascade. Separate thread pools, connection pools per upstream.

## throttling
Apply rate limit. Server-side enforcement. Returns 429.

## degraded-mode
Reduced functionality during partial outage. Show cached results, disable expensive features.

## graceful-degradation
Designing for partial failure. Feature flags to disable, fallback responses, cached data.

## failover
Switch to backup when primary fails. Automatic (Patroni) or manual.

## failback
Return to primary after recovery. Sometimes risky (sync gap).

## rto — Recovery Time Objective
Max acceptable downtime. "RTO 1 hour" = must be back online within 1h.

## rpo — Recovery Point Objective
Max acceptable data loss. "RPO 5 min" = okay to lose last 5 minutes of writes.

## disaster-recovery
Strategy for major outages. Backups, replicated infra, runbooks. Test regularly.

## active-active
Both regions serve traffic. Lowest failover time. Hardest data consistency.

## active-passive
Standby region ready to take over. Simpler. Failover time = warming up.

## pilot-light
DR pattern: minimal standby always running, scale up on failover.

## backup-snapshot
Point-in-time copy. Cloud: EBS snapshots, GCP disk snapshots. Test restores!

## point-in-time-recovery — pitr
Restore DB to specific timestamp. Combines base backup + WAL/binlog replay.

## ransomware-recovery
Restore from off-site backup (3-2-1 rule: 3 copies, 2 media, 1 off-site). Immutable backups (S3 Object Lock).

## aws
Amazon Web Services. Cloud market leader.

## gcp
Google Cloud Platform.

## azure
Microsoft Azure.

## ec2
AWS Elastic Compute Cloud. VMs. Instance families (general, compute-optimized, memory, storage, GPU).

## ecs
AWS Elastic Container Service. Container orchestration. Simpler than EKS, AWS-specific.

## eks
AWS Elastic Kubernetes Service. Managed K8s control plane.

## fargate
AWS serverless containers. No node management. Runs on ECS or EKS.

## lambda
AWS serverless functions. Pay per request + duration. Cold starts. Max 15 min runtime.

## step-functions
AWS state machine for orchestrating Lambda + other services. Long-running workflows.

## sqs-vs-sns-vs-eventbridge
SQS: queue (poll). SNS: pub/sub fanout. EventBridge: routing rules + SaaS events.

## s3
AWS Simple Storage Service. Object store. Strong consistency. Various storage classes.

## s3-intelligent-tiering
Auto-moves objects between storage classes by access patterns.

## s3-glacier
Cold storage. Cheap, retrieval slow + fee.

## s3-versioning
Keep all versions of objects. Protection against accidental delete/overwrite.

## s3-object-lock
WORM (write-once-read-many). Compliance, ransomware protection.

## ebs
AWS Elastic Block Store. Block storage volumes. gp3 default for general use, io2 for high IOPS.

## efs
AWS Elastic File System. NFS. Multi-AZ, shared between instances. Slower than EBS.

## fsx
AWS managed file systems: Lustre (HPC), Windows File Server, ONTAP, OpenZFS.

## rds
AWS managed relational DBs. Postgres, MySQL, MariaDB, Oracle, SQL Server, Aurora.

## aurora
AWS PG/MySQL-compatible DB. Shared storage architecture. 5× faster than vanilla MySQL per AWS claims.

## aurora-serverless
Aurora that scales to 0. Cold starts. Good for spiky workloads.

## dynamodb
AWS managed NoSQL. Key-value or document. Single-digit ms latency at scale. PAY_PER_REQUEST or PROVISIONED.

## dynamodb-streams
Change-data-capture for DynamoDB tables. Triggers Lambda.

## elasticache
AWS managed Redis or Memcached.

## documentdb
AWS managed MongoDB-compatible (older version). NOT real MongoDB.

## redshift
AWS data warehouse. PG-derived. Columnar, MPP.

## athena
AWS Presto. SQL over S3. Pay per query.

## glue
AWS ETL service. Crawlers, jobs, catalog. Spark-based.

## emr
AWS managed Hadoop/Spark cluster.

## msk
AWS managed Kafka.

## sagemaker
AWS ML platform. Training, hosting, notebooks.

## vpc
AWS Virtual Private Cloud. Network isolation.

## subnet
VPC slice in one AZ. Public (route to IGW) or private (route to NAT/Endpoint).

## igw — Internet Gateway
VPC's connection to internet. Stateful, scales automatically.

## nat-gateway
AWS managed NAT for private subnets to reach internet. Expensive ($/hr + per-GB).

## vpc-endpoint
Private access to AWS services without internet. Gateway endpoints (S3, DynamoDB — free), Interface endpoints (rest, paid).

## privatelink
Private service exposure across accounts/VPCs. SaaS pattern.

## transit-gateway
AWS hub-and-spoke routing. Connects many VPCs + on-prem. Replaces VPC peering at scale.

## direct-connect
AWS dedicated network from on-prem. Lower latency, predictable bandwidth.

## route53
AWS DNS. Hosted zones, alias records (no extra lookup), health-check-based routing.

## cloudfront
AWS CDN. Edge locations globally. Lambda@Edge for request transformation.

## elb — Elastic Load Balancer
AWS LB family: ALB (L7), NLB (L4), GLB (gateway, for appliances), CLB (deprecated).

## alb — Application Load Balancer
AWS L7 LB. HTTP/HTTPS/gRPC. Path/host routing, WAF integration.

## nlb — Network Load Balancer
AWS L4 LB. TCP/UDP/TLS. Static IP per AZ, very high throughput, ultra-low latency.

## waf
AWS Web Application Firewall. Rule-based filtering. Managed rule sets (OWASP, bot control).

## shield
AWS DDoS protection. Standard (free, basic), Advanced (paid, $3000/mo, includes WAF).

## iam-aws
AWS Identity and Access Management. Users, groups, roles, policies. See snippet /aws-iam.

## sts
AWS Security Token Service. Issues short-lived credentials. AssumeRole, OIDC federation.

## kms
AWS Key Management Service. Manages encryption keys. CMK + data keys. Integrates with most services.

## secrets-manager
AWS secrets storage with auto-rotation (for supported services: RDS, etc).

## ssm-parameter-store
Cheaper KV for configs/secrets. SecureString uses KMS. Hierarchy via slashes.

## cloudwatch
AWS monitoring. Metrics, Logs, Alarms, Dashboards, Events (now mostly EventBridge).

## cloudwatch-logs-insights
SQL-like query language over CloudWatch Logs.

## cloudtrail
AWS API audit log. Who did what when via API. Send to S3 + Athena for analysis.

## config — AWS Config
Resource compliance tracking. Rules + remediation. Useful for audit.

## guardduty
AWS threat detection. ML on CloudTrail + VPC Flow Logs.

## inspector
AWS vulnerability scanner. EC2 + ECR + Lambda.

## organizations
AWS multi-account management. SCPs (org-wide guardrails), consolidated billing.

## scp-aws — Service Control Policy
AWS Organizations policy. Acts as max-permission boundary. Even root can't exceed.

## landing-zone
Standardized multi-account AWS setup. Control Tower automates this pattern.

## control-tower
AWS service for managing multi-account orgs. Sets up landing zone with guardrails.

## gcp-gke
Google Kubernetes Engine. Managed K8s. Autopilot mode = serverless K8s.

## gcp-cloud-run
Serverless containers. HTTPS endpoint, scales to 0. Knative-based.

## gcp-cloud-functions
GCP serverless functions (similar to AWS Lambda).

## gcp-cloud-sql
Managed Postgres/MySQL/SQL Server.

## gcp-spanner
Globally-distributed strong-consistency SQL. Atomic clocks (TrueTime). Expensive.

## gcp-bigquery
Petabyte-scale SQL data warehouse. Serverless, pay-per-query.

## gcp-pubsub
Global pub/sub messaging.

## gcp-dataflow
Apache Beam runner. Stream + batch processing.

## gcp-firestore
NoSQL document DB. Real-time client SDK. Successor to Datastore.

## gcp-bigtable
Wide-column NoSQL. HBase-compatible API. Linear scale.

## gcp-iam
GCP IAM. Roles (collections of permissions) + Members + Resources. Conditions for fine-grained.

## gcp-vpc
GCP Virtual Private Cloud. Global resource (cross-region). Shared VPC for multi-project.

## gcp-load-balancer
Global LB (with anycast IP) or regional. HTTP(S), TCP, UDP, SSL Proxy.

## azure-aks
Azure Kubernetes Service. Managed K8s.

## azure-functions
Serverless. Multiple plan types (consumption, premium, dedicated).

## azure-app-service
PaaS for web apps. Multiple languages, auto-scale.

## azure-sql
Managed SQL Server. Different deployment models (single, elastic pool, managed instance).

## azure-cosmosdb
Multi-model NoSQL (document, graph, KV, column). Multi-region writes.

## azure-storage
Blob (object), File (SMB), Queue, Table. Account-based.

## azure-service-bus
Enterprise message broker. Queues + topics (pub/sub). FIFO available.

## azure-event-hubs
Kafka-like ingestion. Compatible with Kafka clients.

## azure-event-grid
Event routing. Custom + SaaS events. Push to Functions, Logic Apps.

## azure-ad
Azure Active Directory. Cloud identity. SSO. Used by Microsoft 365.

## entra-id
Rebrand of Azure AD (2023).

## terraform-state
Terraform's source of truth for what exists. Local file or remote (S3, GCS, Terraform Cloud).

## terraform-backend
Where state lives. Remote backend mandatory for teams (S3 + DynamoDB locking).

## terraform-module
Reusable parameterized stack. Local or remote (Terraform Registry, Git).

## terraform-workspace
State namespace within backend. Use for envs (dev/staging/prod) — though separate state files often cleaner.

## terraform-import
Bring existing resources under Terraform management. `terraform import RES_TYPE.NAME id`.

## terraform-plan
Show changes Terraform would make. Read-only.

## terraform-apply
Execute the plan. Modifies infra. State updated.

## terraform-destroy
Remove everything. Dangerous in prod.

## terraform-lifecycle
Per-resource: `prevent_destroy`, `create_before_destroy`, `ignore_changes`.

## tfvars
`.tfvars` file with variable values. Per-environment files common.

## terragrunt
Wrapper around Terraform. DRY composition of state + variables.

## pulumi
IaC in real languages (TypeScript, Python, Go). Alternative to Terraform.

## crossplane
K8s-native multi-cloud IaC. Manage cloud resources via CRDs.

## python
Dynamic interpreted language. CPython reference impl. GIL limits CPU-parallelism. Strong stdlib + ecosystem (NumPy, Pandas, Django).

## gil — Global Interpreter Lock
CPython lock allowing only one thread to execute Python bytecode at a time. Workarounds: multiprocessing, asyncio, C extensions releasing GIL.

## asyncio
Python async I/O framework. `async def` + `await`. Event loop. Use for I/O-bound concurrency.

## pip
Python package installer. `pip install X`. `requirements.txt` for deps lock.

## poetry
Modern Python deps + packaging tool. Lockfile, virtual envs.

## pipenv
Older deps tool (Pipfile). Mostly replaced by Poetry / uv.

## uv
Astral's super-fast Python package manager (Rust). Replaces pip + poetry + venv.

## venv
Python virtual env. Isolated deps per project. `python -m venv .venv && source .venv/bin/activate`.

## pyproject-toml
PEP 518 standard build config for Python. Replaces setup.py for new projects.

## wheel
Python binary package format (.whl). Pre-compiled per platform. Faster install than sdist.

## ruff
Astral's super-fast Python linter (Rust). Drop-in for flake8, isort, black formatting.

## black
Python opinionated formatter. No config, single style.

## mypy
Python static type checker. Type hints + check. Catches many bugs before runtime.

## pylint
Mature Python linter. Slower than ruff. Strict by default.

## flake8
Python linter. Combines pyflakes + pycodestyle + mccabe.

## pytest
Python test framework. Fixtures, parameterize, plugins (mock, asyncio). Default for new projects.

## unittest
Python stdlib test framework. xUnit style. Verbose vs pytest.

## django
Python web framework. Batteries-included (ORM, admin, auth). Synchronous historically, async support added.

## flask
Python micro-framework. Routing + WSGI. Pair with extensions for ORM, auth.

## fastapi
Modern Python web framework. ASGI, type hints → OpenAPI. Fast.

## starlette
ASGI framework. FastAPI built on it. Lower-level alternative to FastAPI.

## sqlalchemy
Python ORM + Core. Most popular DB toolkit. Sessions, queries, migrations (via Alembic).

## alembic
SQLAlchemy migrations. Auto-generate from model diff, then review/edit.

## pandas
Python tabular data lib. DataFrame, Series. Slow for big data; consider Polars.

## numpy
Python n-dim arrays + math. Foundation for ML/data stack.

## scipy
Scientific computing. Optimization, signal processing, stats.

## requests
Python HTTP client. `requests.get(url)`. De facto standard.

## httpx
Modern Python HTTP client. Sync + async. HTTP/2 support.

## golang — Go
Statically-typed compiled language. GC, channels, goroutines. Simple syntax. Backend services common.

## goroutine
Lightweight Go thread (KB stack). Scheduled by Go runtime onto OS threads. `go func() { ... }()`.

## go-channel
Go inter-goroutine communication. `ch := make(chan int)`. Buffered or unbuffered. Closing signals "done".

## select-go
Go's multiplexing: wait on multiple channels. Random choice if multiple ready.

## context-go
Go's `context.Context`. Cancellation + deadline + per-request values. Pass as first param.

## defer
Go statement run at function exit. LIFO order. Use for cleanup (close file, unlock).

## panic
Go's exception. Unwinds stack. Recover via `recover()` in deferred function. Use sparingly.

## go-modules
Go's deps system (since 1.11). `go.mod` + `go.sum`. `go mod tidy` cleans up.

## go-pprof
Built-in profiler. `import _ "net/http/pprof"` exposes endpoints. `go tool pprof` analyzes.

## go-race-detector
`go test -race` / `go run -race`. Finds data races at runtime.

## rust
Systems language with memory safety without GC. Ownership + borrow checker. Steep learning curve.

## cargo
Rust build tool + package manager. `cargo build`, `cargo test`, `Cargo.toml`.

## rust-ownership
Each value has single owner. When owner goes out of scope, value dropped. Prevents leaks + double-free.

## rust-borrow
Reference (`&T` or `&mut T`). Compiler enforces: many readers OR one writer, not both.

## rust-lifetime
Compile-time annotation: how long a reference is valid. Often inferred. Explicit when ambiguous.

## rust-trait
Like interface/typeclass. Default methods, associated types, generics. Trait objects (dyn Trait) for dynamic dispatch.

## tokio
Rust async runtime. Most popular. Multi-threaded scheduler.

## async-std
Alternative Rust async runtime. Less popular than tokio.

## serde
Rust serialization framework. JSON, YAML, MessagePack via derive macros.

## clippy
Rust linter. `cargo clippy`. Catches common mistakes + style issues.

## java
Object-oriented, statically-typed. JVM. Enterprise dominant. Multiple JDK distributions (OpenJDK, Adoptium, GraalVM).

## jvm
Java Virtual Machine. Bytecode runtime. JIT (HotSpot). Many languages target JVM.

## jit-jvm
Just-In-Time compilation. JVM compiles hot methods to native. Tiered: C1 (fast) → C2 (optimal).

## gc-java
Java garbage collection. Algorithms: G1 (default), ZGC (low pause, large heap), Shenandoah, Serial (small heap).

## maven
Java build + deps tool. XML pom.xml. Mature, slow.

## gradle
Java/Kotlin build tool. Groovy/Kotlin DSL. Faster than Maven via incremental + cache.

## spring
Java enterprise framework. Spring Boot for quick start. Heavy but feature-rich.

## kotlin
JVM language. Modern syntax, null-safety, coroutines. Android official.

## scala
JVM language. Functional + OO. Powerful type system. Akka for actor concurrency.

## clojure
JVM Lisp. Immutable data structures by default. STM for concurrency.

## javascript — js
Dynamic language. Originally browser. Node.js for server. Massive ecosystem (npm).

## typescript — ts
JavaScript + types. Compiled to JS. Catches many bugs, better tooling.

## node — Node.js
JS runtime (V8 + libuv). Event loop. Single-threaded JS, multi-threaded I/O.

## npm
Node package manager. `package.json`. `npm install`, `npm test`. Historic deps issues (left-pad).

## yarn
Alternative npm. Faster install historically. v1 vs v2+ (Berry) different.

## pnpm
npm replacement with hard-link store. Faster + less disk usage.

## bun
JS runtime + bundler + npm. Written in Zig. Faster than Node for some workloads.

## deno
Secure JS/TS runtime. Permissions model. ESM-only.

## react
JS UI library. Component-based. Hooks. Virtual DOM. Most popular (2023+).

## vue
JS UI framework. Single-file components. Simpler than React for many.

## angular
JS framework (Google). TypeScript. Heavier than React/Vue.

## svelte
Compiler-based JS UI. No runtime. Smaller bundles.

## next-js
React framework. SSR, file-based routing. Vercel project.

## nuxt
Vue equivalent of Next.js.

## vite
Modern frontend build tool. ESM dev server. Rollup for prod build. Fast.

## webpack
Older bundler. Comprehensive but slow. Mostly being replaced.

## esbuild
Go-based bundler. Very fast. Used by Vite internally.

## c
Systems language. Manual memory. Foundation of OS + many runtimes. Undefined behavior galore.

## cpp — C++
C + classes + templates + much more. Complex but powerful. Modern (C++17/20/23) much safer.

## rust-vs-cpp
Rust: memory safe by default. C++: zero-overhead, mature ecosystem, but UB. Both compile to native code.

## ruby
Dynamic OO language. Rails web framework famous. Smaller ecosystem than Python.

## rails
Ruby on Rails. Convention over configuration. ActiveRecord ORM.

## php
Web-focused dynamic language. WordPress dominant use. PHP 8 much improved.

## elixir
BEAM (Erlang VM) language. Actor model. Phoenix web framework.

## erlang
BEAM language. Telecom origin. Hot code reloading. Fault tolerance via supervisors.

## haskell
Pure functional, lazy, strong types. Steep but powerful.

## lisp
Family of languages (Common Lisp, Scheme, Clojure, Emacs Lisp). Code-as-data (macros).

## ocaml
Functional + OO. Strong types, ML-family. Used by Jane Street, Facebook (Hack).

## swift
Apple's language. iOS, macOS. Memory-safe, modern.

## objective-c
Older Apple language. NeXT/Mac/iOS roots. Mostly replaced by Swift.

## perl
Older scripting language. Regex king. Largely replaced by Python.

## algorithm
Step-by-step problem-solving procedure. Big-O notation for complexity.

## big-o
Asymptotic upper bound. O(1) constant, O(log n), O(n), O(n log n), O(n²), O(2^n). Lower = better.

## big-theta
Tight bound (both upper and lower). Rarely used colloquially.

## big-omega
Asymptotic lower bound.

## sort-bubble
O(n²). Educational only.

## sort-insertion
O(n²) worst, O(n) for nearly-sorted. Small data + linear-search context.

## sort-merge
O(n log n) guaranteed. Stable. O(n) extra space.

## sort-quick
O(n log n) average, O(n²) worst. In-place. Pivot choice matters (randomized for safety).

## sort-heap
O(n log n) guaranteed. In-place. Not stable.

## sort-tim
Hybrid merge + insertion (Python, Java). O(n) for sorted runs.

## sort-radix
O(d·n) for d-digit numbers. Non-comparison based.

## sort-counting
O(n+k) for k distinct values. Only for small-range integers.

## binary-search
O(log n) on sorted array. Half-interval narrowing. Common variant pitfalls: integer overflow, off-by-one.

## linear-search
O(n). When array unsorted or small.

## hash-table
O(1) average lookup/insert. Worst O(n) on collision. Load factor + rehashing.

## linked-list
O(1) insert/delete at head. O(n) random access. Cache-unfriendly.

## doubly-linked-list
Linked list with prev pointers. O(1) delete given node ref.

## array — dynamic array
O(1) random access, O(n) insert middle. Amortized O(1) push (occasional resize).

## stack
LIFO. push/pop O(1). DFS, undo, parser.

## queue
FIFO. enqueue/dequeue O(1). BFS, scheduling.

## deque — double-ended queue
push/pop at both ends. Python `collections.deque`.

## priority-queue
Get min/max O(log n). Insert O(log n). Heap-based usually.

## heap-data
Tree-based priority queue. Min-heap: parent ≤ children. Stored in array.

## tree-binary
Each node ≤ 2 children. Traversals: pre/in/post-order.

## tree-bst — Binary Search Tree
Left subtree < node < right subtree. Unbalanced → O(n) worst.

## tree-avl
Self-balancing BST. Rotation on imbalance. O(log n) all ops.

## tree-red-black
Self-balancing BST. Looser balance than AVL, faster inserts. Used by Linux process tree, Java TreeMap.

## tree-b
B-tree. Many children per node. Optimal for disk (each node = page). Used by DB indexes.

## tree-b-plus
B+ tree variant. All values in leaves, leaves form linked list. Range scans efficient.

## trie
Prefix tree. Each path = string. Used for autocomplete, prefix queries.

## graph
Nodes + edges. Directed or undirected. Weighted or not. Representations: adjacency list (sparse) or matrix (dense).

## bfs — Breadth-First Search
Graph traversal level-by-level. Queue-based. Shortest path on unweighted.

## dfs — Depth-First Search
Graph traversal recursive/stack. Topological sort, cycle detection, connected components.

## dijkstra
Single-source shortest path on non-negative weighted graph. O(E log V) with priority queue.

## bellman-ford
Single-source shortest path, allows negative weights. Detects negative cycles. O(VE).

## floyd-warshall
All-pairs shortest path. O(V³). Use for small dense graphs.

## a-star
Heuristic-guided pathfinding. Faster than Dijkstra when heuristic is good (admissible).

## union-find
Disjoint-set DS. Near-O(1) per op (amortized with path compression + union by rank).

## hash-function
Maps key → fixed-size int. Good: uniform distribution, fast. Cryptographic: also collision-resistant.

## consistent-hash
Hash ring for distributed systems. Adds/removes nodes affect only 1/N keys. Used by caches, sharded DBs.

## bloom-filter
Probabilistic set. False positives OK, no false negatives. Saves space vs hash set when checking membership before expensive op.

## hyperloglog
Probabilistic cardinality estimation. ~1.5KB for billions of values. Used by Redis, Druid.

## count-min-sketch
Probabilistic frequency counting. Stream processing.

## skip-list
Probabilistic alternative to balanced tree. Simpler implementation. Used by Redis sorted sets.

## lru-cache
Least Recently Used eviction. HashMap + DoublyLinkedList for O(1) get/put. Common interview question.

## lfu-cache
Least Frequently Used. More complex than LRU. Often hybrid (LRU within frequency bucket).

## arc — Adaptive Replacement Cache
Self-tuning between LRU + LFU. Used by ZFS.

## dp — dynamic programming
Optimal substructure + overlapping subproblems. Memoization (top-down) or tabulation (bottom-up).

## greedy
Make locally optimal choice each step. Works only when proven correct (e.g. Dijkstra).

## divide-and-conquer
Split → solve subproblems → combine. Merge sort, FFT, Karatsuba multiplication.

## backtracking
DFS with pruning. N-queens, sudoku, knapsack.

## branch-and-bound
Backtracking with bounds to prune. Integer programming.

## np-complete
Class of problems: in NP + at least as hard as any NP problem. SAT, knapsack, TSP. No polynomial algorithm known.

## p-vs-np
Famous open problem. P = polynomial-time solvable, NP = polynomial-time verifiable. P=NP would break crypto.

## reduction
Transform problem A → problem B. If solving B solves A, A "reduces to" B.

## np-hard
At least as hard as NP-complete problems. May not be in NP.

## np
Problems verifiable in polynomial time. P ⊆ NP. Open if equal.

## complexity-class
Group of problems by resource bound. P, NP, PSPACE, EXPTIME.

## monolith
Single deployable codebase. Simple to start, harder to scale teams. Refactor to microservices is hard.

## microservices
Many independently-deployable services. Coordination overhead, networked failures. See snippets.

## soa — Service-Oriented Architecture
Predecessor of microservices. Heavier (ESB, SOAP). Enterprise.

## esb — Enterprise Service Bus
SOA middleware (MuleSoft, BizTalk). Centralized integration. Largely deprecated for event-driven.

## event-driven
Architecture: components communicate via events. Loose coupling. Hard to debug (eventual consistency).

## event-sourcing
Store events instead of current state. Replay = derive state. Audit log built-in.

## cqrs — Command Query Responsibility Segregation
Separate read + write models. Often paired with event sourcing.

## ddd — Domain-Driven Design
Software design centered on domain model. Bounded contexts, aggregates, ubiquitous language.

## bounded-context
DDD: explicit boundary for a model. Each microservice = one bounded context (ideal).

## aggregate
DDD: cluster of objects treated as one unit. Single entry point (root). Transactional consistency boundary.

## hexagonal
Architecture: domain at center, ports + adapters. Aka Ports & Adapters or Clean Architecture.

## clean-arch
Architecture: dependency direction inward. Entities > Use cases > Interface adapters > Frameworks.

## onion-arch
Similar to hexagonal/clean. Concentric layers, dependencies inward.

## mvc — Model-View-Controller
Classic UI pattern. Separate data, presentation, control.

## mvvm
Model-View-ViewModel. Two-way binding between View and ViewModel. WPF, Angular.

## flux-pattern
Unidirectional data flow (Facebook). Inspired Redux.

## redux
JS state management. Single store, reducers, actions. Immutable updates.

## graphql-vs-rest
GraphQL: flexible queries, single endpoint, complex caching. REST: simple, HTTP-native, easy caching.

## rest-vs-grpc
REST: human-readable, browser-friendly. gRPC: binary, streaming, code-gen, internal services.

## openapi
API spec format (formerly Swagger). YAML/JSON. Generate clients + docs from spec.

## json-schema
Vocabulary for JSON validation. Used in OpenAPI, configs.

## tdd — Test-Driven Development
Write test first → fail → write code → pass → refactor. Discipline.

## bdd — Behavior-Driven Development
Cucumber/Gherkin. Tests in natural language. Stakeholder-readable.

## unit-test
Test single function/class in isolation. Fast (<100ms). Mock external deps.

## integration-test
Test multiple components together (real DB, real services). Slower. Use Testcontainers for ephemeral deps.

## e2e — End-to-End test
Test full system as user would. Playwright, Cypress for web. Slowest, most brittle.

## smoke-test
Quick sanity check. Run after deploy. "Does it boot? Can I login?"

## load-test
Sustained traffic to validate capacity. k6, locust, wrk, JMeter.

## stress-test
Beyond expected load. Find breaking point + degradation curve.

## chaos-test
Inject failures (kill pods, network partition, latency). Validate resilience.

## fuzz-test
Random/generated inputs. Find crashes, edge cases. AFL, libfuzzer, cargo-fuzz.

## property-test
Test invariants over generated inputs. QuickCheck, Hypothesis, proptest.

## mutation-test
Mutate code, re-run tests. Tests that still pass = weak. Stryker, Pitest.

## coverage
% of code executed by tests. Branch coverage > line coverage. 70% sane target.

## ci — Continuous Integration
Merge frequently to main. Auto-build + test on each merge. Catches breaks early.

## cd — Continuous Delivery
Code always deployable. Manual gate to push.

## continuous-deployment
Every passing build auto-deploys to prod. Highest discipline required.

## blue-green
Two prod envs. Switch all traffic at once. Instant rollback.

## canary-deploy
Send small % to new version. Monitor. Scale if good, abort if bad.

## rolling-deploy
Replace instances gradually. K8s Deployment default.

## feature-flag
Code path enabled/disabled at runtime. Decouple deploy from release. LaunchDarkly, Unleash, Flagsmith.

## a-b-test
Show different versions to user buckets. Statistical analysis of business metrics.

## dark-launch
Deploy code that runs but isn't visible. Verify no perf regression before exposing.

## shadow-traffic
Send copy of prod traffic to new version. Compare behavior without affecting users.

## monorepo
Single repo for many projects. Nx, Bazel, Turborepo. Pros: atomic changes. Cons: large, tooling needed.

## polyrepo
One repo per project. Independent versioning. Dep upgrades manual.

## subtree
Git mechanism: include another repo as subdirectory. Simpler than submodule.

## submodule
Git pointer to specific commit of another repo. Track separately. Painful UX.

## gitflow
Branching model: develop, feature, release, hotfix branches. Heavy. Often overkill.

## trunk-based
All work on main (trunk). Short-lived feature branches. Pairs with feature flags.

## github-flow
Simpler than gitflow. Branch from main, PR, merge.

## semver — Semantic Versioning
MAJOR.MINOR.PATCH. Major: breaking. Minor: feature. Patch: fix.

## calver
Calendar versioning. e.g. 2024.05.01. Ubuntu, JetBrains tools.

## changelog
Human-readable list of changes per version. Keep-a-Changelog standard format.

## conventional-commits
Commit message format: `feat: add X`, `fix: ...`. Tooling can generate changelog + version bumps.

## release-train
Scheduled release cadence. e.g. weekly. Features either ship by Friday or wait.

## hotfix
Urgent prod fix, bypassing normal release cadence.

## rollback
Revert to previous version. Should be fast + safe. Test rollback procedure regularly!

## migration-db
Schema change script. Versioned, applied in order. Tools: Flyway, Liquibase, Alembic, Atlas.

## migration-zero-downtime
Apply DB changes without app downtime. Expand-and-contract: new column nullable → backfill → app uses → drop old.

## expand-contract
Schema migration pattern. Add new (compatible with old code) → switch reads/writes → drop old.

## strangler-fig
Migration pattern: gradually replace old system, route by feature. Named after Martin Fowler's metaphor.

## refactor
Change code structure without changing behavior. Tests crucial. Small, frequent commits.

## tech-debt
Quick solutions accruing cleanup cost. Track explicitly. Pay down regularly.

## code-review
Peer review of changes. Catches bugs + spreads knowledge. Tools: GitHub PRs, GitLab MRs, Gerrit.

## pair-programming
Two devs, one keyboard. Driver + navigator. Catches issues live + onboards juniors.

## mob-programming
Whole team on one screen. Extreme of pair programming. Used for design/onboarding.

## ddos — Distributed Denial of Service
Many attackers flood target. Mitigations: CDN, rate limit, WAF, anti-DDoS service (Cloudflare, Shield).

## dos — Denial of Service
Single source DoS. Easier to block.

## sql-injection
Untrusted input concatenated into SQL. Mitigations: parameterized queries, ORM, prepared statements.

## xss — Cross-Site Scripting
Untrusted input rendered as HTML/JS. Mitigations: escape on output, CSP, HttpOnly cookies.

## csrf — Cross-Site Request Forgery
Other site triggers request as your user. Mitigation: CSRF token, SameSite cookies.

## clickjacking
Trick user into clicking via iframe overlay. Mitigation: X-Frame-Options, frame-ancestors CSP.

## ssrf — Server-Side Request Forgery
App fetches URL controlled by attacker. Risk: hits internal services (metadata IMDS). Mitigations: allowlist, disable redirects, block private IPs.

## xxe — XML External Entity
XML parser fetches external entity. Risk: file disclosure, SSRF. Mitigation: disable DTD processing.

## rce — Remote Code Execution
Attacker runs arbitrary code on server. Worst CVE class. Often from deserialization, command injection.

## lfi — Local File Inclusion
Read arbitrary file via path traversal. Mitigation: validate paths, chroot.

## rfi — Remote File Inclusion
Include code from URL. Almost always RCE. Rare in modern frameworks.

## privilege-escalation
Gain higher privileges than intended. Local (user→root) or vertical (user→admin).

## zero-trust
Security model: never trust, always verify. No implicit network trust. mTLS, IAM, device posture.

## defense-in-depth
Multiple security layers. Single failure ≠ breach. WAF + app validation + DB user perms.

## least-privilege
Principle: grant minimum necessary perms. Limit blast radius.

## blast-radius-security
Damage scope of compromise. Architectural goal: minimize.

## ddos-amplification
UDP-based attacks where small request → big response (DNS, NTP, memcached). Amplification factor 10×-50000×.

## tls-pinning
Hardcode expected cert/pubkey in client. Defeats CA compromise. Used by mobile apps.

## hpkp
HTTP Public Key Pinning. Browser pin via header. Deprecated due to footgun risk.

## hash-rainbow-table
Precomputed hash→input table. Defeated by salt + slow hashes.

## salt
Random per-record value mixed into hash input. Prevents rainbow table attacks.

## pepper
Server-wide secret added to hash input. Stored separately from DB.

## bcrypt
Slow password hash. Cost factor adjustable. Default for password storage.

## argon2
Modern password hash. Memory-hard. Recommended for new systems.

## scrypt
Memory-hard hash. Used by Litecoin, password storage.

## pbkdf2
Older password hash. Iterations parameter. Use Argon2/bcrypt for new code.

## hmac
Hash-based MAC. Symmetric key signing. Common for API request signing.

## ecdsa
Elliptic-curve digital signature. Smaller keys than RSA for same security. SSH ed25519 uses Ed25519 variant.

## rsa
Asymmetric crypto. Pubkey encryption + signing. Recommended 4096-bit. Slower than EC.

## aes
Symmetric block cipher. 128/192/256-bit keys. Industry standard.

## chacha20
Stream cipher. Faster than AES on devices without AES-NI. Used by WireGuard, TLS.

## sha-256
256-bit hash. Currently secure. Used for integrity, signatures.

## sha-3
Newer hash standard (Keccak). Different design vs SHA-2. Less common in practice.

## blake2
Modern hash. Faster than SHA-2. blake3 newer (parallelism).

## md5
128-bit hash. BROKEN for security. Still OK for non-security checksums.

## sha-1
160-bit hash. Collisions demonstrated. Phase out. Git uses but moving to SHA-256.

## twofa — Two-Factor Authentication
Something you know + something you have/are. TOTP, hardware key, biometric.

## totp
Time-based One-Time Password. RFC 6238. 6-digit codes from app (Authy, Google Authenticator).

## fido2
Hardware security key standard. YubiKey, etc. Phishing-resistant.

## webauthn
Browser API for FIDO2. Passwordless future.

## passkey
WebAuthn marketed to users. Phishing-resistant, cross-device.

## oauth-flow-auth-code
Most common. Browser → auth → code → backend exchanges for tokens.

## oauth-flow-pkce
PKCE = Proof Key for Code Exchange. Auth code flow for public clients (mobile, SPA). Replaces implicit flow.

## oauth-flow-implicit
DEPRECATED. Tokens in fragment. XSS-prone.

## oauth-flow-password
DEPRECATED. App handles password. Violates separation.

## oauth-flow-client-credentials
Machine-to-machine. Client ID + secret. No user.

## oauth-flow-device-code
CLI / smart TV. Show code on small device, login on big one.

## oauth-scope
Granular permission. `read:emails`, `write:posts`. Show to user at consent.

## sso — Single Sign-On
One login for many apps. SAML, OIDC. Reduces password sprawl.

## sso-vs-federated-id
SSO is the user experience. Federation is the protocol. SAML/OIDC are federation methods.

## scim
System for Cross-domain Identity Management. User provisioning protocol. Auto-create accounts from IdP.

## just-in-time-access
Grant elevated access for specific task/duration. Auto-revoke. Tools: Teleport, Boundary.

## bastion
Hardened jump host. Only way to reach prod. Heavy logging. Modern alternative: identity-aware proxies.

## tailscale
WireGuard-based mesh VPN. Easy setup. Identity-based access (Google/Microsoft SSO).

## wireguard-vs-openvpn
WireGuard: faster, simpler. OpenVPN: mature, more configurable.

## ipsec-vs-wireguard
IPsec: standard, complex. WireGuard: modern, simpler, kernel-included.

## zerologon
2020 critical Windows AD CVE. Auth bypass via netlogon. Caused patching panic.

## log4shell
Log4j 2021 RCE. Just-by-logging exploit. Massive impact, multi-week mitigation rush.

## heartbleed
2014 OpenSSL bug. Leaked memory via TLS heartbeat. Forced industry rotation.

## spectre-meltdown
2018 CPU side-channel CVEs. Speculative execution leaks. Microcode + kernel mitigations cost perf.

## shellshock
2014 Bash CVE. Code execution via env vars. Affected CGI scripts.

## struts2-rce
2017 Equifax breach root cause. Apache Struts deserialization.

## solarwinds
2020 supply-chain attack. Malicious update to network monitoring tool. Affected gov + Fortune 500.

## sast — Static Application Security Testing
Analyze code without running. Semgrep, SonarQube, CodeQL.

## dast — Dynamic Application Security Testing
Test running app. Burp, ZAP. Finds runtime issues SAST misses.

## sca — Software Composition Analysis
Analyze deps for known CVEs. Dependabot, Snyk, Trivy.

## iast — Interactive Application Security Testing
Instrument running app + observe. Hybrid SAST/DAST.

## rasp — Runtime Application Self-Protection
Monitor + block at runtime. Inside app. e.g. Contrast.

## bug-bounty
Pay external researchers for findings. HackerOne, Bugcrowd. Define scope clearly.

## responsible-disclosure
Researcher reports privately, gives time to fix, then publishes. Usually 90 days.

## cve — Common Vulnerabilities and Exposures
Public ID for security flaw. CVE-2024-XXXXX. Tracked by MITRE.

## cvss
Common Vulnerability Scoring System. 0-10 severity. v3 most common.

## kev — Known Exploited Vulnerabilities catalog
CISA's list of CVEs actively exploited. Priority for patching.

## sbom — Software Bill of Materials
List of all deps + versions. SPDX, CycloneDX formats. Required by EO 14028 for US gov contracts.

## supply-chain-attack
Compromise via upstream dep. SolarWinds, event-stream npm package, codecov.

## reproducible-build
Same source → byte-identical binary. Detects tampering. Bazel, Nix.

## signing
Cryptographic signature on artifact. cosign for containers, GPG for packages.

## attestation
Signed claim about an artifact (built by X, tested by Y). SLSA framework.

## slsa
Supply-chain Levels for Software Artifacts. Google framework. Levels 1-4.

## sigstore
Open-source signing for software supply chain. Cosign, Rekor, Fulcio.

## acm-cert-manager
K8s operator for cert management. Auto-issues from Let's Encrypt/Vault.

## opa-gatekeeper
Policy-as-code for K8s. CRD-based constraints + templates.

## kyverno
K8s policy engine. YAML-based (vs Rego). Mutating + validating + generating policies.

## falco
Runtime security via eBPF + syscall monitoring. Alerts on suspicious behavior.

## tracee
Aqua Security's eBPF-based runtime tracing. Falco alternative.

## anchore
Container image scanner. Compliance + CVEs.

## clair
Container image vulnerability scanner. CoreOS origin.

## trivy
All-in-one scanner: containers, IaC, secrets, deps, K8s. Aqua Security.

## grype
Anchore's CVE scanner. Pairs with Syft (SBOM).

## syft
SBOM generator. Aqua/Anchore. JSON/SPDX/CycloneDX outputs.

## docker-content-trust
Sign + verify docker images. Notary-based. Cosign is modern replacement.

## image-pull-secret
K8s Secret with registry credentials. Required for private images.

## seccomp
Linux syscall filtering. Restrict what container can call. Docker default profile blocks ~40 syscalls.

## apparmor-docker
LSM profile for containers. Path-based access control. Default Docker profile.

## selinux-docker
LSM for containers. Label-based MAC. RHEL/CentOS default.

## rootless
Run container as non-root user. Podman default. Better security.

## userns — user namespaces
Map UID 0 in container to non-root on host. Containment if container compromised.

## privileged-container
`--privileged` Docker flag. Disables most isolation. AVOID unless necessary (Docker-in-Docker, etc).

## capability-linux
Granular permissions split from root. `CAP_NET_ADMIN`, `CAP_SYS_ADMIN`. Drop to minimum.

## immutable-infra
Replace, don't patch. Servers ephemeral. AMIs, containers, GitOps.

## phoenix-server
Server you can kill + recreate. Opposite of pet servers.

## cattle-vs-pets
Servers as cattle (interchangeable) vs pets (named, cared for). Cattle wins at scale.

## twelve-factor
12-factor app methodology (Heroku). Stateless, env config, port binding, etc.

## hipaa
US health data privacy law. PHI handling. Encryption at rest + transit. BAAs.

## gdpr
EU data protection law. Consent, right to erasure, data portability. Fines up to 4% revenue.

## ccpa
California Consumer Privacy Act. US state equivalent of GDPR.

## pci-dss
Payment Card Industry Data Security Standard. Required for handling card data.

## sox — Sarbanes-Oxley
US public company financial controls. IT general controls section affects engineering.

## soc2
Service Organization Control 2. Trust criteria: security, availability, confidentiality. Required by enterprise customers.

## iso27001
Information security mgmt standard. Cert process. Common internationally.

## fedramp
US gov cloud authorization. Required for federal customers. Long, expensive process.

## hitrust
Health Information Trust Alliance. Common framework combining HIPAA + others.

## ransomware
Malware encrypting victim's files for ransom. Mitigations: immutable backups, EDR, network segmentation.

## phishing
Social engineering via fake messages. #1 initial access vector.

## spear-phishing
Targeted phishing. Researched victim, personalized message.

## bec — Business Email Compromise
Phishing impersonating exec. Wire transfer fraud. Massive losses globally.

## smishing
SMS-based phishing.

## vishing
Voice-based phishing. Often paired with email for legitimacy.

## insider-threat
Authorized user causing damage. Malicious or negligent.

## edr — Endpoint Detection and Response
Monitor + respond on endpoints. CrowdStrike, SentinelOne, Defender.

## xdr — Extended Detection and Response
EDR + network + email + cloud correlation.

## siem — Security Information and Event Management
Log aggregation + correlation + alerting. Splunk, Sentinel, ELK + plugins.

## soar — Security Orchestration, Automation, Response
Automate SOC playbooks. Phantom, Demisto.

## ueba — User and Entity Behavior Analytics
ML on access patterns. Detect anomalies (user logging in from new country).

## dlp — Data Loss Prevention
Detect/block sensitive data exfiltration. Egress monitoring.

## casb — Cloud Access Security Broker
Visibility + policy enforcement for SaaS use. Shadow IT detection.

## sase — Secure Access Service Edge
Convergence of SD-WAN + security (ZTNA, SWG, CASB). Single cloud-delivered service.

## sd-wan
Software-defined WAN. Replaces MPLS with internet + smart routing.

## ztna — Zero Trust Network Access
Replaces VPN. Per-app access based on identity + posture.

## sandbox
Isolated execution env. Run untrusted code/files. Cuckoo, Joe Sandbox.

## honeypot
Decoy system attracting attackers. Detect + study.

## red-team
Offensive simulation. Test defenses by attacking like real adversary.

## blue-team
Defensive team. SOC, incident response.

## purple-team
Red + blue working together. Faster iteration.

## tabletop
Discussion-based exercise. Walk through hypothetical incident.

## ttp — Tactics, Techniques, Procedures
Attacker behavior patterns. MITRE ATT&CK framework catalogs.

## mitre-attack
Knowledge base of adversary TTPs. Reference for threat modeling, detection rules.

## threat-model
Identify potential threats + mitigations. STRIDE, PASTA frameworks.

## stride
Spoofing, Tampering, Repudiation, Info disclosure, DoS, Elevation. Microsoft threat model.

## pasta
Process for Attack Simulation + Threat Analysis. 7 stages.

## iocs — Indicators of Compromise
Forensic artifacts (file hashes, IPs, domains) suggesting breach.

## ioc
Same as IOCs (singular).

## tip — Threat Intelligence Platform
Aggregate + share IOCs. MISP, Anomali.

## stix-taxii
Structured threat intel format + transport. Industry standard.

## opensearch-security
Open-source plugin for OpenSearch. ES paid feature without paying Elastic.

## kibana-spaces
ES feature: namespace dashboards by team/env. OS dashboards equivalent.

## elastalert
Alerts on Elasticsearch queries. Bridge to PagerDuty/Slack.

## graylog
Log aggregation + analysis. Java. ES-backed.

## papertrail
Hosted syslog/log aggregation. SolarWinds.

## sumologic
Hosted log analytics platform.

## loggly
Cloud log mgmt (SolarWinds).

## logz-io
Hosted ELK-as-a-service.

## datadog-apm
Datadog application performance monitoring. Distributed tracing.

## newrelic-apm
NewRelic equivalent.

## dynatrace
APM with full-stack auto-instrumentation. ML-based RCA.

## appdynamics
Cisco APM. Java-strong.

## honeycomb
Observability tool focused on high-cardinality fields. BubbleUp for RCA.

## lightstep
Tracing platform. ServiceNow acquired.

## elastic-apm
ES APM. Bundled in Elastic Cloud.

## sentry-performance
Sentry's APM add-on. Bridges errors + perf.

## rollbar-stack
Stacktrace-focused error tracker. Better grouping.

## airbrake
Errbit-style error tracker. Long history.

## bugfender
Mobile-focused log SaaS.

## crashlytics
Firebase mobile crash reporting. Free.

## sentry-replay
Session replay (DOM events) on error. Reproduce user state.

## fullstory
Session replay SaaS. Heavy. Privacy concerns.

## logrocket
Session replay + error tracker. JS-heavy SPAs.

## hotjar
Heatmap + recording. User research tool.

## datadog-rum
Real User Monitoring. Page load, errors, vitals.

## newrelic-browser
Browser monitoring agent. Similar to Datadog RUM.

## webpagetest
Open-source web perf testing. Detailed waterfall, filmstrip.

## lighthouse
Google Chrome perf/SEO/a11y audit. CI integration via lighthouse-ci.

## core-web-vitals
Google metrics: LCP, FID/INP, CLS. SEO ranking factor.

## lcp — Largest Contentful Paint
Time until biggest visible element loads. Target <2.5s.

## fid — First Input Delay
Time from first interaction to handler. Target <100ms. Being replaced by INP.

## inp — Interaction to Next Paint
Replaces FID 2024. Sustained responsiveness metric.

## cls — Cumulative Layout Shift
Sum of unexpected layout shifts. Target <0.1.

## ttfb — Time To First Byte
Server response time + network. Target <800ms.

## fcp — First Contentful Paint
First text/image appears. Target <1.8s.

## fmp
First Meaningful Paint. Deprecated, use LCP.

## tti — Time To Interactive
JS bundle parsed + handlers attached. Target <3.8s.

## tbt — Total Blocking Time
Sum of main-thread blocking >50ms. Lab analog of FID.

## sli-vs-slo-vs-sla
SLI = measurement, SLO = target, SLA = contract.

## golden-signals-extra
Saturation = how full is the system? Queue depth, IOPS used / max, connection pool utilization.

## chaos-monkey
Netflix tool. Randomly terminates instances in prod. Forced resilience.

## chaos-gorilla
Netflix tool. Disables AZ.

## chaos-kong
Netflix tool. Disables region.

## simian-army
Netflix collection of chaos tools. Largely retired in favor of Gremlin etc.

## gremlin
Commercial chaos engineering platform. Fault injection.

## chaos-mesh
K8s-native chaos. CRDs for pod-kill, network-partition, IO-faults.

## litmus
Open-source K8s chaos engineering. CNCF.

## stress-test-tools
k6 (load), JMeter (full-featured), wrk (HTTP perf), vegeta (constant-rate).

## locust
Python load testing. Distributed. Behavior-based scripts.

## k6
Modern load testing. JS scripts. Easy integration with Grafana.

## jmeter
Apache load testing. GUI + CLI. Heavy but featureful.

## wrk
Simple HTTP benchmarking. Single binary.

## vegeta
Go HTTP load tool. Constant rate, JSON output.

## ab — Apache Bench
Simplest HTTP benchmarking. Available everywhere.

## artillery
Node.js load testing. Multi-protocol (HTTP, WebSocket, Socket.io).

## gatling
Scala-based load testing. Code-first. Good reports.

## drill
Rust load tester. Simple YAML scenarios.

## hyperfine
Command-line benchmarking. Statistical. `hyperfine 'cmd-a' 'cmd-b'`.

## perf-trace
Profile via syscall trace. `perf trace -p PID`.

## perf-stat
CPU counter stats. `perf stat -e cycles,instructions cmd`.

## flamegraph-tool
`flamegraph.pl` from Brendan Gregg. Aggregate stack samples → SVG flame.

## offcpu-flamegraph
Flame graph of WHERE process is blocked (waiting), not running.

## differential-flamegraph
Compare two flame graphs. Red = increased time, blue = decreased.

## brendangregg
Performance expert (Netflix → Intel). Book "Systems Performance". Tools + methods.

## use-tool
Brendan Gregg's checklist applying USE method per resource.

## ai-ml-glossary-divider
The following are ML/AI terms commonly relevant in tech interviews.

## ml-supervised
Training with labeled data. Predict label from features. Regression (continuous) or classification (discrete).

## ml-unsupervised
Find structure in unlabeled data. Clustering (K-means), dimensionality reduction (PCA, t-SNE).

## ml-reinforcement
Agent learns by reward signal. RL. AlphaGo, ChatGPT RLHF.

## ml-overfitting
Model memorizes training data. Bad generalization. Mitigations: more data, regularization, dropout, cross-validation.

## ml-underfitting
Model too simple. Misses patterns. More features, more complex model.

## ml-bias-variance
Bias = systematic error. Variance = sensitivity to training set. Tradeoff: simple models bias-prone, complex variance-prone.

## ml-train-test-split
Hold out portion for evaluation. 70/15/15 (train/val/test) typical.

## ml-cross-validation
K-fold: split N ways, train on K-1, test on 1, rotate. Better than single split for small data.

## ml-regularization
Penalize complex models. L1 (sparse), L2 (smooth). Drop-out for neural nets.

## ml-feature-engineering
Transform raw data into model-friendly form. Categorical encoding, normalization, polynomial features.

## ml-feature-store
Centralized feature mgmt. Reuse across teams + offline/online consistency. Feast, Tecton.

## ml-embedding
Vector representation of discrete object (word, item, user). Trained or pre-trained.

## word2vec
Classic word embedding model (2013, Google). CBOW or Skip-gram.

## glove
Embedding model (Stanford). Co-occurrence statistics.

## bert
Transformer language model (2018, Google). Bidirectional context. Pretrained then fine-tuned.

## gpt
Transformer language model family (OpenAI). Generative. Decoder-only.

## transformer
Neural net architecture (2017). Self-attention. Foundation of modern NLP.

## attention
Mechanism for weighing input parts. Scaled dot-product attention in transformer.

## llm — Large Language Model
Big transformer trained on text. GPT, Claude, Llama, Gemini.

## rag — Retrieval-Augmented Generation
LLM + vector DB retrieval. Ground answers in source docs. Reduces hallucinations.

## vector-db
DB for similarity search over embeddings. Pinecone, Weaviate, Qdrant, Chroma, pgvector.

## ann — Approximate Nearest Neighbor
Fast similarity search (sub-linear). HNSW, IVF algorithms.

## hnsw
Hierarchical Navigable Small World. Graph-based ANN. Pinecone, Weaviate use.

## fine-tuning
Continue training pre-trained model on task-specific data. LoRA reduces cost.

## lora — Low-Rank Adaptation
Parameter-efficient fine-tuning. Add small trainable matrices. Faster, less memory.

## quantization
Reduce model precision (FP32 → INT8/4). Smaller, faster, slight accuracy loss.

## distillation
Train small model to mimic big one. Smaller, faster, similar accuracy.

## pruning
Remove unimportant weights. Sparser, faster.

## prompt-engineering
Craft inputs to LLMs for better output. Few-shot, chain-of-thought, role prompts.

## few-shot
Provide examples in prompt. LLM follows pattern.

## chain-of-thought
"Let's think step by step". Improves reasoning on multi-step problems.

## react-agent
Reasoning + Acting. LLM observes, thinks, acts (tool call), repeats.

## function-calling
LLM outputs structured JSON for tool use. OpenAI / Anthropic feature.

## tool-use
LLM calls external functions. Search, calculator, code execution.

## tokenizer
Splits text into tokens for LLM. BPE most common. ~4 chars/token English, more for other langs.

## context-window
Max tokens LLM can see. GPT-4 32k-128k, Claude 200k+, Gemini 1M+.

## hallucination
LLM generates plausible but false content. Mitigation: RAG, lower temperature, citations.

## temperature
LLM sampling parameter. 0 = greedy/deterministic, higher = more random.

## top-p
Sampling: choose from tokens summing to probability p. Nucleus sampling.

## top-k
Sampling: choose from top k tokens. Less common than top-p.

## stop-tokens
Strings LLM stops at. Useful for structured output.

## system-prompt
Initial instruction setting LLM behavior. "You are a helpful assistant..."

## prompt-injection
User input overrides system prompt. "Ignore previous instructions and..."

## jailbreak
Bypass LLM safety guardrails. Adversarial prompts.

## rlhf — Reinforcement Learning from Human Feedback
Train reward model from preferences → fine-tune via RL. ChatGPT, Claude technique.

## sft — Supervised Fine-Tuning
Continue training on labeled examples. Easier than RLHF, weaker results.

## dpo — Direct Preference Optimization
Skip reward model in RLHF. Often comparable results, simpler.

## moe — Mixture of Experts
Sparse model: activate subset of params per input. Mixtral, Gemini use. Faster inference.

## chunk-mlops
Split docs into chunks for embedding. Trade off context vs retrieval precision.

## embedding-model
Maps text → vector. Sentence-BERT, OpenAI text-embedding-3, Cohere embed.

## sentence-transformers
Library for sentence embeddings. Many pre-trained models.

## inference-server
Serves model predictions. NVIDIA Triton, TensorFlow Serving, TorchServe, BentoML.

## model-registry
Store + version models. MLflow, Weights & Biases.

## mlflow
Open-source ML lifecycle platform. Experiments, model registry, deployment.

## wandb
Weights & Biases. Experiment tracking. Better UX than MLflow per many.

## kubeflow
K8s-native ML platform. Pipelines, training, serving. Complex.

## seldon-core
K8s model serving. Canary, A/B, explainers.

## kserve
K8s model serving (formerly KFServing). Knative-based.

## vllm
Fast LLM serving. PagedAttention. High throughput.

## tensorrt-llm
NVIDIA optimized LLM inference.

## llama-cpp
CPU/GPU LLM inference. Quantized models. Easy local use.

## ollama
Local LLM running tool. Wraps llama.cpp. Easy CLI.

## openllm
Self-hosted LLM serving. BentoML.

## langchain
Python framework for LLM apps. Many integrations. Sometimes over-abstracted.

## llamaindex
RAG-focused framework. Better than LangChain for RAG per many.

## autogen
Microsoft multi-agent framework. Agents conversing.

## crewai
Multi-agent framework with role-based design.

## drift — model drift
Model performance degrading over time (data shifts). Monitor + retrain.

## data-drift
Input distribution changes. PSI, KL divergence to detect.

## concept-drift
Y|X relationship changes. Harder to detect than data drift.

## label-leakage
Future info in training data. Model "cheats". Common ML failure.

## class-imbalance
One class dominant in training data. Use class weights, SMOTE, balanced sampling.

## confusion-matrix
TP, FP, TN, FN counts. Foundation for precision, recall, F1.

## precision-ml
TP / (TP + FP). Of predicted positives, % actually positive.

## recall-ml
TP / (TP + FN). Of actual positives, % found.

## f1
2 × P × R / (P + R). Harmonic mean of precision + recall.

## auc-roc
Area Under ROC Curve. Single number for binary classifier quality. 0.5 random, 1.0 perfect.

## ml-cross-entropy-loss
Loss function for classification. Penalizes confident wrong predictions heavily.

## mse — Mean Squared Error
Regression loss. Penalizes large errors quadratically.

## mae — Mean Absolute Error
Regression loss. Robust to outliers vs MSE.

## ml-batch-normalization
Normalize activations per mini-batch. Stabilizes training, allows higher learning rates.

## ml-dropout
Randomly zero activations during training. Regularization.

## ml-residual-connection
Skip connections in deep nets. Enables very deep models (ResNet).

## ml-attention-self
Each token attends to all others in input. Quadratic complexity.

## ml-attention-cross
Decoder attends to encoder output. Translation, image captioning.

## transformer-encoder
Reads input, produces representations. BERT.

## transformer-decoder
Generates output autoregressively. GPT.

## transformer-encoder-decoder
Original architecture. T5, machine translation models.

## gradient-descent
Optimize by following negative gradient. Step size = learning rate.

## sgd — Stochastic Gradient Descent
Update on mini-batches. Noisier but faster + better generalization than full-batch.

## adam
Adaptive optimizer. Most popular. Combines momentum + per-parameter learning rates.

## adagrad
Per-parameter learning rate based on history. Good for sparse data.

## rmsprop
Exponential decay of past gradients. Predecessor to Adam.

## learning-rate
Step size in gradient descent. Too high diverges, too low slow. Schedules: warmup + cosine decay.

## batch-size
# samples per gradient update. Larger = faster but worse generalization (often).

## epoch
One pass through training data. Train for several epochs typically.

## early-stopping
Stop training when val loss stops improving. Prevent overfitting.

## hyperparameter
Set by you, not learned. Learning rate, layer count, regularization strength.

## hpo — Hyperparameter Optimization
Search hyperparameter space. Grid, random, Bayesian (Optuna, Hyperopt), genetic.

## ablation
Remove components to measure their contribution. Standard in ML research.

## sgd-momentum
Add velocity term to gradient. Smooths updates, escapes plateaus.

## warmup
Gradually increase learning rate at start of training.

## cosine-decay
Learning rate schedule following cosine curve. Smooth decrease.

## ml-data-leak
Train data leaks into test. Common: time-based splits where test info leaks back via features.

## time-series-split
For temporal data: train on past, test on future. Don't shuffle.

## stratified-split
Maintain class proportions in train/test. Important for imbalanced data.

## active-learning
Model picks which examples to label. Reduce labeling cost.

## semi-supervised
Some labeled + lots of unlabeled. Self-training, consistency regularization.

## self-supervised
Learn from raw data via pretext tasks (next-token prediction, masked LM).

## contrastive-learning
Learn embeddings by pulling similar pairs together, dissimilar apart. SimCLR.

## triplet-loss
Anchor, positive, negative. Used for face/object recognition.

## meta-learning
Learn to learn. Adapt quickly to new tasks. MAML.

## federated-learning
Train across decentralized devices without centralizing data. Google keyboard, healthcare.

## differential-privacy
Mathematical privacy guarantee. Noise added to outputs to obscure individuals. Apple, Google use.

## ml-explainability
Why did model predict X? SHAP, LIME for feature attribution.

## shap
SHapley Additive exPlanations. Per-feature contribution to prediction.

## lime
Local Interpretable Model-agnostic Explanations. Approximate locally.

## ml-fairness
Bias mitigation across protected groups. Demographic parity, equalized odds.

## a-b-test-vs-ml
A/B test is product science. ML evaluation is offline accuracy. Both needed.

## ml-canary
Deploy new model to small % of traffic. Monitor business KPIs.

## ml-shadow
New model runs alongside, compares outputs offline. No user impact.

## ml-rollback
Switch back to previous model. Needs versioning + serving infra support.

## ml-monitoring
Track input distribution (drift), output distribution, business KPIs, model latency.

## prompt-template
Reusable LLM prompt with variable slots. LangChain PromptTemplate.

## prompt-caching
Cache common prompt prefixes to save tokens + latency. Anthropic prompt caching.

## token-usage
LLM cost = input + output tokens. Monitor per request, set budgets.

## openai
LLM provider. GPT family. Pioneer of consumer LLMs (ChatGPT).

## anthropic
LLM provider. Claude family. Constitutional AI approach.

## google-deepmind
LLM provider. Gemini family. Multi-modal strong.

## mistral
LLM provider. Open-weight models. Mixtral MoE.

## meta-llama
Open-weight LLM. Llama 2, 3, 4. Strong open-source baseline.

## huggingface
Model hub + libs (transformers, datasets, accelerate). De facto open-source AI hub.

## groq-hw
Inference HW (LPU). Very fast LLM serving. (NOT to be confused with Grok.)

## together-ai
Open-source LLM hosting platform.

## replicate
LLM/image model API service.

## fireworks-ai
Fast OSS LLM inference.

## openrouter
Multi-provider LLM API gateway.

## bedrock
AWS LLM service. Claude, Llama, Titan, others.

## vertex-ai
GCP ML platform. Gemini access.

## azure-openai
Microsoft's OpenAI offering. GPT models on Azure infra.

## ml-edge
Run model on edge device (phone, IoT). TFLite, CoreML, ONNX Runtime.

## tflite
TensorFlow Lite. Mobile/embedded inference.

## coreml
Apple framework. iOS on-device ML.

## onnx
Open Neural Network Exchange. Cross-framework format.

## onnx-runtime
ONNX execution. Cross-platform, optimized.

## pytorch
Meta's ML framework. Most popular for research.

## tensorflow
Google's ML framework. Production-strong (TF Serving, TFX).

## jax
Google's high-perf array library. NumPy + autodiff + GPU/TPU. Research.

## hf-transformers
HuggingFace transformers lib. Easy pretrained model loading.

## sklearn
Scikit-learn. Classical ML in Python.

## xgboost
Gradient boosting. Often wins tabular Kaggle competitions.

## lightgbm
Microsoft's gradient boosting. Faster than XGBoost.

## catboost
Yandex's gradient boosting. Native categorical handling.

## random-forest
Ensemble of decision trees. Simple, often surprisingly good baseline.

## decision-tree
Tree of feature splits. Interpretable, weak alone. Foundation for forests + boosting.
