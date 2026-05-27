# Commands Reference v1

Tool-specific command cheatsheets. Each entry: `## <tool>` heading, body is a list of common commands with one-line purpose.
Parser: split on `\n## `.

## kubectl-basics
- `kubectl get pods` — list pods in current namespace
- `kubectl get pods -A` — all namespaces
- `kubectl get pods -n NAMESPACE -o wide` — verbose
- `kubectl get pods -w` — watch live
- `kubectl get pods -l app=foo` — filter by label
- `kubectl describe pod NAME` — events at bottom are gold
- `kubectl logs POD` — current logs
- `kubectl logs POD -f` — tail follow
- `kubectl logs POD --previous` — last container's logs (crashed)
- `kubectl logs POD -c CONTAINER` — specific container
- `kubectl exec -it POD -- bash` — interactive shell
- `kubectl exec POD -- env` — print env vars

## kubectl-apply
- `kubectl apply -f manifest.yaml` — declarative create/update
- `kubectl apply -f dir/` — all yamls in directory
- `kubectl delete -f manifest.yaml` — symmetric delete
- `kubectl diff -f manifest.yaml` — preview changes
- `kubectl rollout status deployment/X` — wait for rollout
- `kubectl rollout history deployment/X` — past revisions
- `kubectl rollout undo deployment/X` — rollback to previous
- `kubectl rollout undo deployment/X --to-revision=3` — specific rev
- `kubectl rollout restart deployment/X` — restart all pods

## kubectl-debug
- `kubectl get events --sort-by=.lastTimestamp` — cluster-wide events
- `kubectl top pod` — resource usage (needs metrics-server)
- `kubectl top node` — node usage
- `kubectl debug -it POD --image=busybox` — ephemeral debug container
- `kubectl port-forward svc/X 8080:80` — local → cluster
- `kubectl auth can-i create pods --as=USER` — RBAC test
- `kubectl explain deployment.spec.template` — schema docs
- `kubectl get pod POD -o yaml | less` — full manifest

## kubectl-create
- `kubectl create deployment NAME --image=IMG`
- `kubectl run NAME --image=IMG` — quick pod (no controller)
- `kubectl create configmap NAME --from-file=path`
- `kubectl create secret generic NAME --from-literal=KEY=VAL`
- `kubectl create namespace foo`
- `kubectl create job NAME --image=IMG` — one-shot
- `kubectl create cronjob NAME --image=IMG --schedule='*/5 * * * *'`

## kubectl-context
- `kubectl config get-contexts` — list known clusters
- `kubectl config use-context NAME` — switch
- `kubectl config current-context`
- `kubectl config set-context --current --namespace=foo` — default ns
- `kubectx` — third-party fast context switcher
- `kubens` — third-party namespace switcher

## helm
- `helm repo add bitnami https://charts.bitnami.com/bitnami`
- `helm repo update`
- `helm search repo postgres` — discover charts
- `helm install NAME chart -n NAMESPACE` — deploy
- `helm install NAME chart -f values.yaml --set key=val`
- `helm upgrade NAME chart -f values.yaml`
- `helm upgrade --install NAME chart` — atomic create-or-update
- `helm list -A` — all releases
- `helm history NAME` — past revisions
- `helm rollback NAME 3` — revert to revision
- `helm uninstall NAME -n NAMESPACE`
- `helm template NAME chart -f values.yaml` — render without apply
- `helm get manifest NAME` — what's deployed
- `helm get values NAME` — what values used

## docker
- `docker ps` — running containers
- `docker ps -a` — including stopped
- `docker images` — local images
- `docker run --rm -it IMAGE bash` — interactive throw-away
- `docker run -d -p 8080:80 --name web IMAGE` — background, port map
- `docker logs CONTAINER -f` — follow logs
- `docker exec -it CONTAINER bash` — shell into running
- `docker inspect CONTAINER` — full state JSON
- `docker stats` — live resource usage
- `docker stop CONTAINER`
- `docker rm CONTAINER` — remove stopped
- `docker rm -f CONTAINER` — force-remove running
- `docker rmi IMAGE` — remove image
- `docker image prune -a` — clean unused
- `docker system prune -a --volumes` — nuke everything unused
- `docker build -t NAME:TAG .` — build from Dockerfile
- `docker push NAME:TAG` — push to registry
- `docker pull NAME:TAG` — fetch
- `docker tag SRC NEW` — alias an image
- `docker cp CONTAINER:/src local` — copy out
- `docker save IMAGE > out.tar` — export image
- `docker load -i out.tar` — import

## docker-compose
- `docker compose up -d` — start in background
- `docker compose down` — stop + remove
- `docker compose down -v` — also remove volumes
- `docker compose logs -f SVC` — tail one service
- `docker compose exec SVC bash`
- `docker compose ps` — services status
- `docker compose pull` — update images
- `docker compose build` — rebuild
- `docker compose restart SVC`
- `docker compose run --rm SVC cmd` — one-shot

## git
- `git status`
- `git diff` — unstaged changes
- `git diff --staged` — what's about to commit
- `git add -p` — interactive hunk staging
- `git commit -m 'msg'`
- `git commit --amend` — replace last (only if not pushed!)
- `git log --oneline -20` — last 20 commits
- `git log --graph --all --oneline` — branch topology
- `git log -p -- path/file` — file history with diffs
- `git blame file` — who changed each line
- `git show HEAD~3` — view specific commit
- `git stash` — save WIP
- `git stash pop` — restore
- `git stash list` — multiple stashes

## git-branch
- `git branch` — local branches
- `git branch -a` — all incl remote
- `git checkout BRANCH` — switch
- `git switch BRANCH` — newer syntax
- `git switch -c NEW` — create + switch
- `git checkout -b NEW` — older syntax for same
- `git branch -d BRANCH` — delete (safe)
- `git branch -D BRANCH` — force delete
- `git push -u origin BRANCH` — first push, set upstream

## git-merge-rebase
- `git merge BRANCH` — merge into current
- `git merge --no-ff BRANCH` — force merge commit
- `git rebase main` — replay current onto main
- `git rebase -i HEAD~5` — interactive rewrite last 5
- `git rebase --continue` — after fixing conflicts
- `git rebase --abort`
- `git cherry-pick HASH` — copy commit
- `git revert HASH` — create undo commit

## git-remote
- `git remote -v` — list remotes
- `git fetch` — get latest, don't merge
- `git fetch --all`
- `git pull` — fetch + merge
- `git pull --rebase` — fetch + rebase (cleaner history)
- `git push` — to upstream
- `git push --force-with-lease` — safer force push
- `git remote prune origin` — clean stale branches

## git-recovery
- `git reflog` — every HEAD movement (your time machine)
- `git reset --hard HEAD@{2}` — undo to N reflog steps back
- `git reset --soft HEAD~1` — undo commit, keep changes staged
- `git reset --mixed HEAD~1` — undo commit, keep changes unstaged
- `git checkout -- file` — discard uncommitted changes
- `git restore file` — newer syntax
- `git clean -fd` — remove untracked files+dirs (DESTRUCTIVE)
- `git fsck --lost-found` — find orphaned commits

## git-bisect
- `git bisect start`
- `git bisect bad HEAD` — known broken
- `git bisect good v1.2.0` — known working
- (git checks out midpoint, you test, mark)
- `git bisect bad` — current commit broken
- `git bisect good` — current commit fine
- `git bisect reset` — done
- `git bisect run ./test.sh` — fully automated

## ssh
- `ssh user@host` — basic connect
- `ssh -p 2222 user@host`
- `ssh -i ~/.ssh/key user@host` — specific key
- `ssh -L 8080:localhost:80 user@host` — local port-forward
- `ssh -R 9000:localhost:9000 user@host` — reverse forward
- `ssh -D 1080 user@host` — SOCKS5 proxy
- `ssh -J bastion user@target` — proxy-jump
- `ssh -o StrictHostKeyChecking=no user@host` — skip key check (CI use)
- `ssh-keygen -t ed25519 -C 'comment'` — generate keypair
- `ssh-copy-id user@host` — install pubkey
- `ssh-add ~/.ssh/key` — add to agent
- `ssh-keygen -R host` — remove stale host key
- `~/.ssh/config` aliases — `Host alias\n  HostName real\n  User foo`

## scp-rsync
- `scp file user@host:/path` — copy file
- `scp -r dir user@host:/path` — recursive
- `scp -P 2222 file user@host:/path` — port
- `rsync -avz src/ user@host:/dst/` — archive + compress
- `rsync -avz --delete src/ dst/` — delete extras in dst
- `rsync -avzn src/ dst/` — dry-run
- `rsync -avz --progress src/ dst/` — show progress
- `rsync -e 'ssh -i key' src dst` — custom ssh

## tmux
- `tmux new -s name` — new named session
- `tmux ls` — list sessions
- `tmux attach -t name` — reattach
- `tmux kill-session -t name`
- `Ctrl-b d` — detach
- `Ctrl-b c` — new window
- `Ctrl-b ,` — rename window
- `Ctrl-b w` — list windows
- `Ctrl-b %` — vertical split
- `Ctrl-b "` — horizontal split
- `Ctrl-b arrow` — move between panes
- `Ctrl-b z` — zoom pane fullscreen
- `Ctrl-b [` — copy mode (q to exit)
- `Ctrl-b ]` — paste

## ssh-tunneling-examples
- `ssh -L 3306:db.internal:3306 bastion` — reach internal DB locally
- `ssh -L 5901:localhost:5901 user@host` — VNC tunnel
- `autossh -M 0 -f -N -L 8080:localhost:80 host` — persistent tunnel
- `ssh -fN -L 8080:localhost:80 host` — daemonize

## curl
- `curl URL` — fetch + print body
- `curl -I URL` — HEAD only
- `curl -v URL` — verbose (headers, TLS)
- `curl -L URL` — follow redirects
- `curl -o file URL` — save to file
- `curl -O URL` — save with remote filename
- `curl -d 'foo=bar' URL` — POST form
- `curl -d '@body.json' -H 'Content-Type: application/json' URL` — POST JSON file
- `curl -X PUT URL` — different method
- `curl -H 'Authorization: Bearer TOKEN' URL`
- `curl -u user:pass URL` — basic auth
- `curl --resolve example.com:443:1.2.3.4 URL` — override DNS
- `curl -k URL` — skip cert verify (DANGEROUS in prod)
- `curl --insecure URL` — same as -k
- `curl -w '%{http_code} %{time_total}\n' URL` — custom output
- `curl --connect-timeout 5 --max-time 10 URL` — timeouts
- `curl -X PATCH -d @body.json URL`

## jq
- `jq '.field' file` — extract field
- `jq '.items[]' file` — iterate array
- `jq '.items[].name' file` — field per item
- `jq -r '.field' file` — raw string (no quotes)
- `jq '.[] | select(.active)' file` — filter
- `jq 'map(.name)' file` — transform array
- `jq '.[0:5]' file` — slice
- `jq 'group_by(.team)' file`
- `jq -s '.' file1 file2` — slurp into array
- `jq '. + {new: "val"}' file` — add field
- `jq 'del(.field)' file` — remove field
- `jq -c '.[]' file` — compact JSONL output
- `cat ndjson | jq -s '.'` — JSONL → JSON array

## yq
- `yq '.field' file.yaml` — mikefarah/yq
- `yq -i '.field = "value"' file.yaml` — in-place edit
- `yq '.items[].name' file.yaml`
- `yq -o=json '.' file.yaml` — convert to JSON

## grep
- `grep 'pattern' file`
- `grep -r 'pat' dir/` — recursive
- `grep -i 'pat' file` — case-insensitive
- `grep -v 'pat' file` — invert match
- `grep -c 'pat' file` — count matches
- `grep -n 'pat' file` — line numbers
- `grep -l 'pat' *.log` — only file names with match
- `grep -E 'a|b' file` — extended regex
- `grep -P 'lookbehind' file` — Perl regex (GNU)
- `grep -A 3 -B 2 'pat' file` — N lines after/before context
- `grep -C 5 'pat' file` — context (around)
- `grep -o 'PAT' file` — print only match
- `grep --color=auto 'pat'` — highlight (often default)

## ripgrep
- `rg 'pattern'` — recurse from cwd, respects .gitignore
- `rg -i 'pat'` — case-insensitive
- `rg -tpython 'pat'` — only Python files
- `rg -n 'pat'` — line numbers (default)
- `rg --hidden 'pat'` — include hidden files
- `rg -uu 'pat'` — disable .gitignore
- `rg -A 3 -B 2 'pat'` — context
- `rg --json 'pat'` — structured output

## awk
- `awk '{print $1}' file` — first field
- `awk -F, '{print $3}' csv` — custom field sep
- `awk 'NR > 1' file` — skip header
- `awk '$3 > 100' file` — filter by value
- `awk '/PAT/' file` — pattern match
- `awk '{sum += $1} END {print sum}' file` — sum column
- `awk '{print NF}' file` — fields per line
- `awk '!seen[$0]++' file` — dedup preserving order
- `ls -la | awk '{print $5, $9}'` — multi-field

## sed
- `sed 's/old/new/' file` — first occurrence per line
- `sed 's/old/new/g' file` — all per line
- `sed -i 's/old/new/g' file` — in-place (BSD needs -i '')
- `sed -n '10,20p' file` — print lines 10-20
- `sed '/pat/d' file` — delete lines matching
- `sed '5d' file` — delete line 5
- `sed -e 's/a/b/' -e 's/c/d/' file` — multiple
- `sed 's|/|\\|g' file` — alternative delimiter
- `sed -E 's/(foo|bar)/X/g' file` — extended regex

## find
- `find . -name '*.log'` — by name
- `find . -iname '*.LOG'` — case-insensitive
- `find . -type f` — files only
- `find . -type d` — dirs only
- `find . -size +100M` — bigger than 100MB
- `find . -mtime -7` — modified within 7 days
- `find . -mtime +30` — modified more than 30 days ago
- `find . -newer ref-file`
- `find . -name '*.tmp' -delete` — delete matches
- `find . -name '*.log' -exec rm {} \;` — run cmd per match
- `find . -name '*.log' -exec rm {} +` — batched (faster)
- `find . -name '*.log' -print0 | xargs -0 rm` — safer for spaces
- `find . -type f -not -path '*/.git/*'` — exclude
- `find . -empty` — empty files/dirs
- `find . -perm 0644` — by permission

## xargs
- `find ... | xargs cmd` — basic
- `find ... -print0 | xargs -0 cmd` — null-separated (safe for spaces)
- `xargs -n 1 cmd` — one arg per invocation
- `xargs -P 4 cmd` — 4 parallel
- `xargs -I{} cmd {} arg` — placeholder
- `seq 100 | xargs -P 10 -I{} curl host/{}`

## tar
- `tar czf out.tgz dir/` — create gzipped
- `tar xzf in.tgz` — extract gzipped
- `tar tzf in.tgz` — list contents
- `tar cjf out.tbz2 dir/` — bzip2
- `tar cJf out.txz dir/` — xz (best ratio)
- `tar c dir | zstd -T0 > out.tar.zst` — zstd parallel
- `tar xf in.tgz -C /target/dir`
- `tar --exclude='*.log' czf out.tgz dir/`

## systemctl
- `systemctl status nginx`
- `systemctl start nginx`
- `systemctl stop nginx`
- `systemctl restart nginx`
- `systemctl reload nginx` — SIGHUP, less disruptive
- `systemctl enable nginx` — start on boot
- `systemctl disable nginx`
- `systemctl is-active nginx`
- `systemctl is-enabled nginx`
- `systemctl list-units --failed`
- `systemctl list-unit-files`
- `systemctl daemon-reload` — reload unit files after edit
- `systemctl cat nginx` — view current unit file
- `systemctl edit nginx` — override

## journalctl
- `journalctl -u nginx` — service logs
- `journalctl -u nginx -f` — follow
- `journalctl -u nginx --since '1 hour ago'`
- `journalctl -u nginx --until '5 min ago'`
- `journalctl -u nginx -n 100` — last 100 lines
- `journalctl -u nginx -p err` — errors and above
- `journalctl -b` — current boot
- `journalctl -b -1` — previous boot
- `journalctl -k` — kernel only
- `journalctl --disk-usage`
- `journalctl --vacuum-size=500M` — shrink

## ps-top
- `ps aux` — all processes (BSD)
- `ps -ef` — all processes (long)
- `ps aux --sort=-%mem | head` — top memory
- `ps aux --sort=-%cpu | head` — top CPU
- `ps -p PID -o etime` — uptime
- `pstree -p` — process tree with PIDs
- `top -p PID` — single process
- `top -H -p PID` — threads of process
- `htop` — interactive (F9 kill, F5 tree)
- `pidof nginx`
- `pgrep -f 'pattern'` — find PID by pattern
- `pkill -f 'pattern'` — kill by pattern

## kill-signals
- `kill PID` — SIGTERM (15), graceful
- `kill -9 PID` — SIGKILL, force
- `kill -1 PID` — SIGHUP, often "reload config"
- `kill -2 PID` — SIGINT, like Ctrl-C
- `kill -3 PID` — SIGQUIT, Java thread dump
- `kill -l` — list signals
- `killall nginx` — by name

## df-du
- `df -h` — disk free (human)
- `df -i` — inode usage
- `df -hT` — include filesystem type
- `du -sh dir/` — total size
- `du -hx --max-depth=1 / | sort -h` — top-level breakdown
- `du -ah dir | sort -hr | head` — top files
- `ncdu /` — interactive TUI
- `lsblk` — block devices tree
- `mount | column -t` — mounted FS

## free-vmstat
- `free -h` — memory summary
- `free -h -s 1` — refresh every 1s
- `vmstat 1` — VM + IO + CPU every 1s
- `vmstat 1 10` — 10 samples
- `cat /proc/meminfo` — detailed memory
- `slabtop` — kernel slab cache
- `smem -tk` — better memory accounting

## iostat-iotop
- `iostat -xz 1` — extended disk stats
- `iostat -xz 1 10` — 10 samples
- `iotop` — per-process IO
- `iotop -oP` — only active, processes (not threads)
- `pidstat -d 1` — per-process disk
- `sar -d 1` — disk history

## ss-netstat
- `ss -tlnp` — TCP listen with PID
- `ss -tnp` — TCP all with PID
- `ss -unp` — UDP
- `ss -tn state established` — current connections
- `ss -s` — summary
- `ss -tnH | wc -l` — count connections
- `netstat -tlnp` — old equivalent of ss -tlnp
- `lsof -i :443` — what listens on 443
- `lsof -i tcp` — all TCP

## tcpdump
- `tcpdump -i any` — all interfaces
- `tcpdump -i eth0 port 443`
- `tcpdump -i any host 1.2.3.4`
- `tcpdump -nn` — don't resolve names (faster)
- `tcpdump -w file.pcap` — write to file
- `tcpdump -r file.pcap` — read from file
- `tcpdump -X` — print hex + ascii
- `tcpdump -s 0` — capture full packets (not truncated)
- `tcpdump 'tcp[tcpflags] & (tcp-syn) != 0'` — SYN only

## iptables-nftables
- `iptables -L -n -v` — list rules
- `iptables -t nat -L -n -v`
- `iptables -A INPUT -p tcp --dport 22 -j ACCEPT`
- `iptables -D INPUT 3` — delete rule 3
- `iptables-save > rules.txt`
- `iptables-restore < rules.txt`
- `nft list ruleset` — modern equivalent
- `nft add table inet filter`
- `nft 'add chain inet filter input { type filter hook input priority 0; }'`

## ufw
- `ufw status`
- `ufw enable`
- `ufw allow 22/tcp`
- `ufw allow from 10.0.0.0/8 to any port 3306`
- `ufw delete allow 22/tcp`
- `ufw deny 6379`

## firewall-cmd
- `firewall-cmd --get-active-zones`
- `firewall-cmd --list-all`
- `firewall-cmd --add-port=80/tcp --permanent`
- `firewall-cmd --add-service=https --permanent`
- `firewall-cmd --reload`

## openssl
- `openssl s_client -connect host:443 -servername host` — TLS debug
- `openssl s_client -showcerts -connect host:443` — full chain
- `openssl x509 -in cert.pem -text -noout` — parse cert
- `openssl x509 -in cert.pem -noout -dates` — validity
- `openssl x509 -in cert.pem -noout -issuer -subject`
- `openssl req -new -key key.pem -out req.csr` — generate CSR
- `openssl req -text -in req.csr -noout` — view CSR
- `openssl rsa -in key.pem -text -noout`
- `openssl genrsa -out key.pem 4096`
- `openssl ecparam -genkey -name prime256v1 -out key.pem`
- `openssl rand -hex 32` — random bytes
- `openssl rand -base64 32`
- `openssl dgst -sha256 file`
- `openssl enc -aes-256-cbc -salt -in file -out file.enc`
- `openssl pkcs12 -in cert.pfx -nokeys` — extract from PKCS12

## dig
- `dig example.com`
- `dig +short example.com`
- `dig +trace example.com`
- `dig @8.8.8.8 example.com` — specific resolver
- `dig MX example.com`
- `dig TXT example.com`
- `dig -x 1.2.3.4` — reverse lookup
- `dig ANY example.com`
- `dig +tcp example.com` — force TCP

## prom-promql
- `up{job="api"} == 0` — service down
- `rate(http_requests_total[5m])` — qps
- `sum by (instance) (rate(node_cpu_seconds_total{mode!="idle"}[5m]))`
- `histogram_quantile(0.99, sum by (le) (rate(http_duration_bucket[5m])))` — p99
- `sum(rate(http_requests_total[5m])) by (status_code)`
- `100 * sum(rate(http_requests_total{status_code=~"5.."}[5m])) / sum(rate(http_requests_total[5m]))` — error rate %
- `predict_linear(disk_free[1h], 4*3600)` — disk free in 4h
- `node_memory_MemAvailable_bytes / node_memory_MemTotal_bytes` — mem % free
- `up < 1` — instances down right now

## logql-loki
- `{job="api"}` — all from job
- `{job="api"} |= "error"` — contains "error"
- `{job="api"} != "health"` — doesn't contain
- `{job="api"} |~ "5\\d\\d"` — regex match (5xx)
- `{job="api"} | json | level="error"` — parse JSON, filter
- `{job="api"} | logfmt | response_time > 500` — logfmt parse
- `rate({job="api"} |= "error" [5m])` — error rate
- `sum by (level) (rate({job="api"} | json [5m]))`

## awscli-basics
- `aws configure` — set creds + region
- `aws sts get-caller-identity` — who am I
- `aws ec2 describe-instances`
- `aws ec2 describe-instances --filters 'Name=tag:Env,Values=prod'`
- `aws s3 ls`
- `aws s3 ls s3://bucket/path/`
- `aws s3 cp file s3://bucket/path/`
- `aws s3 sync local/ s3://bucket/path/`
- `aws s3 rm --recursive s3://bucket/path/`
- `aws s3 presign s3://bucket/file --expires-in 3600`

## awscli-ec2
- `aws ec2 start-instances --instance-ids i-XXX`
- `aws ec2 stop-instances --instance-ids i-XXX`
- `aws ec2 reboot-instances --instance-ids i-XXX`
- `aws ec2 terminate-instances --instance-ids i-XXX`
- `aws ec2 describe-instances --query 'Reservations[].Instances[].[InstanceId, State.Name, Tags[?Key==\`Name\`].Value|[0]]' --output table`
- `aws ec2 get-console-output --instance-id i-XXX`

## awscli-iam
- `aws iam list-users`
- `aws iam list-roles`
- `aws iam get-role --role-name X`
- `aws iam list-attached-role-policies --role-name X`
- `aws iam simulate-principal-policy --policy-source-arn ARN --action-names s3:GetObject`

## awscli-eks
- `aws eks list-clusters`
- `aws eks update-kubeconfig --name CLUSTER --region REGION`
- `aws eks describe-cluster --name CLUSTER`
- `aws eks list-nodegroups --cluster-name CLUSTER`

## gcloud-basics
- `gcloud auth login`
- `gcloud auth list`
- `gcloud config set project PROJECT_ID`
- `gcloud projects list`
- `gcloud compute instances list`
- `gcloud compute ssh INSTANCE --zone ZONE`
- `gcloud container clusters get-credentials CLUSTER --region REGION`
- `gcloud storage ls gs://bucket/`
- `gcloud storage cp local gs://bucket/`

## az-basics
- `az login`
- `az account show`
- `az account set --subscription SUB_ID`
- `az group list -o table`
- `az vm list -o table`
- `az aks list -o table`
- `az aks get-credentials --resource-group RG --name CLUSTER`

## psql
- `psql -h host -U user -d db`
- `psql -d db` — local
- `\l` — list databases
- `\c db` — connect to db
- `\dt` — list tables
- `\dt+` — with sizes
- `\d table` — describe
- `\d+ table` — with detail
- `\du` — list users
- `\dn` — schemas
- `\df` — functions
- `\timing on` — show query times
- `\x on` — expanded display
- `\copy table to '/tmp/out.csv' csv header`
- `\copy table from '/tmp/in.csv' csv header`
- `\watch 5` — re-run last query every 5s

## pg-admin
- `SELECT version();`
- `SELECT current_database(), current_user;`
- `SELECT pg_size_pretty(pg_database_size('mydb'));`
- `SELECT pg_size_pretty(pg_total_relation_size('mytable'));`
- `SELECT * FROM pg_stat_activity WHERE state != 'idle';`
- `SELECT * FROM pg_stat_replication;`
- `SELECT * FROM pg_locks WHERE NOT granted;`
- `SELECT * FROM pg_stat_user_indexes WHERE idx_scan = 0;` — unused indexes
- `SELECT * FROM pg_stat_user_tables ORDER BY n_dead_tup DESC LIMIT 10;` — bloated
- `VACUUM ANALYZE table;`
- `REINDEX TABLE CONCURRENTLY table;`
- `ANALYZE table;`

## mysql-cli
- `mysql -h host -u user -p db`
- `SHOW DATABASES;`
- `USE db;`
- `SHOW TABLES;`
- `DESCRIBE table;` or `SHOW CREATE TABLE table\G`
- `SHOW PROCESSLIST;` — active queries
- `SHOW FULL PROCESSLIST;`
- `SHOW VARIABLES LIKE '%buffer%';`
- `SHOW STATUS LIKE 'Threads_%';`
- `SHOW ENGINE INNODB STATUS\G`
- `SHOW SLAVE STATUS\G` (MySQL <8.0) / `SHOW REPLICA STATUS\G`
- `EXPLAIN SELECT ...;`
- `EXPLAIN ANALYZE SELECT ...;` (MySQL 8.0+)

## redis-cli
- `redis-cli -h host -p 6379`
- `redis-cli ping` → PONG
- `redis-cli info` — server stats
- `redis-cli info stats` — only stats section
- `redis-cli info memory`
- `redis-cli info replication`
- `redis-cli dbsize`
- `redis-cli monitor` — live cmd stream (DON'T use in prod)
- `redis-cli --latency` — latency to server
- `redis-cli --latency-history` — over time
- `redis-cli --bigkeys` — find biggest keys
- `redis-cli --memkeys` — memory by key
- `redis-cli --scan --pattern 'user:*'`
- `redis-cli debug sleep 5` — block server (testing)
- `redis-cli config get maxmemory`
- `redis-cli config set maxmemory 1gb`
- `redis-cli flushdb` — DANGER: empty current db
- `redis-cli flushall` — DANGER: empty all dbs

## mongosh
- `mongosh "mongodb://host:27017"`
- `show dbs`
- `use mydb`
- `show collections`
- `db.users.find()`
- `db.users.find({active: true}).limit(5)`
- `db.users.findOne({_id: ObjectId("...")})`
- `db.users.countDocuments({})`
- `db.users.insertOne({...})`
- `db.users.updateOne({_id: X}, {$set: {f: v}})`
- `db.users.deleteMany({inactive: true})`
- `db.users.createIndex({email: 1}, {unique: true})`
- `db.users.getIndexes()`
- `db.users.aggregate([{$match: {...}}, {$group: {...}}])`
- `db.users.explain('executionStats').find({...})`

## kafka-cli
- `kafka-topics.sh --bootstrap-server LOC --list`
- `kafka-topics.sh --bootstrap-server LOC --describe --topic NAME`
- `kafka-topics.sh --bootstrap-server LOC --create --topic NAME --partitions 6 --replication-factor 3`
- `kafka-console-consumer.sh --bootstrap-server LOC --topic NAME --from-beginning`
- `kafka-console-producer.sh --bootstrap-server LOC --topic NAME`
- `kafka-consumer-groups.sh --bootstrap-server LOC --list`
- `kafka-consumer-groups.sh --bootstrap-server LOC --describe --group GROUP`
- `kafka-consumer-groups.sh --bootstrap-server LOC --reset-offsets --group GROUP --topic NAME --to-earliest --execute`

## terraform-cli
- `terraform init` — set up backend + download providers
- `terraform plan` — preview changes
- `terraform plan -out=plan.tfplan`
- `terraform apply plan.tfplan`
- `terraform apply -auto-approve` (avoid in prod)
- `terraform destroy`
- `terraform state list`
- `terraform state show RES_TYPE.NAME`
- `terraform state rm RES_TYPE.NAME` — forget without destroy
- `terraform import RES_TYPE.NAME id`
- `terraform output`
- `terraform fmt -recursive`
- `terraform validate`
- `terraform workspace list/select/new`
- `terraform refresh` — sync state with reality

## ansible-cli
- `ansible all -m ping` — connectivity check
- `ansible all -m setup` — gather facts
- `ansible-playbook playbook.yml`
- `ansible-playbook playbook.yml --check` — dry-run
- `ansible-playbook playbook.yml --diff` — show changes
- `ansible-playbook playbook.yml -l host1,host2` — limit hosts
- `ansible-playbook playbook.yml --tags TAG`
- `ansible-playbook playbook.yml -e 'var=val'`
- `ansible-playbook playbook.yml -i inventory.yml -K -k`
- `ansible-vault encrypt secrets.yml`
- `ansible-vault edit secrets.yml`
- `ansible-galaxy install -r requirements.yml`

## github-cli
- `gh auth login`
- `gh repo clone owner/repo`
- `gh repo create name --public/--private`
- `gh issue list`
- `gh issue create --title TITLE --body BODY`
- `gh issue view NUM`
- `gh pr list`
- `gh pr create --title TITLE --body BODY`
- `gh pr checkout NUM`
- `gh pr view NUM`
- `gh pr merge NUM --squash`
- `gh run list` — actions runs
- `gh run watch` — live workflow
- `gh release create v1.0 --notes 'changes'`
- `gh secret set NAME` — set repo secret
- `gh api repos/owner/repo/issues` — generic API call

## perf-tools
- `perf top` — system-wide cpu profile
- `perf top -p PID`
- `perf record -F 99 -p PID -g -- sleep 30`
- `perf report` — view recording
- `perf stat -e cycles,instructions cmd` — counter stats
- `perf trace cmd` — strace-like via perf
- `perf list` — available events

## ebpf-bcc
- `execsnoop` — log every exec
- `opensnoop` — log every open
- `tcpconnect` — log TCP connect
- `tcpaccept` — log accept
- `biolatency` — block IO latency histogram
- `funccount 'cache*'` — count function calls matching pattern
- `argdist -C 'r::SyS_open():int:$retval'` — distribution of retval
- `runqlat` — scheduler run queue latency

## bpftrace
- `bpftrace -l 'tracepoint:*'` — list tracepoints
- `bpftrace -e 'kprobe:vfs_read { @[comm] = count(); }'`
- `bpftrace -e 'tracepoint:syscalls:sys_enter_openat { printf("%s %s\n", comm, str(args->filename)); }'`
- `bpftrace -e 'profile:hz:99 /pid == 1234/ { @[ustack] = count(); }'`

## flamegraph
- `perf record -F 99 -p PID -g -- sleep 30`
- `perf script | stackcollapse-perf.pl | flamegraph.pl > out.svg`
- `flamegraph` Rust tool: `cargo install flamegraph; flamegraph -- cmd`

## strace-ltrace
- `strace cmd`
- `strace -p PID` — attach
- `strace -f -p PID` — follow forks
- `strace -e openat cmd` — filter syscalls
- `strace -c cmd` — summary stats
- `strace -tt cmd` — timestamps
- `strace -o log.txt cmd`
- `ltrace cmd` — library calls
- `ltrace -p PID`

## lsof
- `lsof -p PID`
- `lsof -i :443` — what listens on port
- `lsof -i tcp` — all TCP
- `lsof -i @1.2.3.4` — connections to/from host
- `lsof /var/log/file.log` — who has this open
- `lsof | grep deleted` — files held but deleted
- `lsof -nP -iTCP -sTCP:LISTEN`
- `lsof -u USER` — by user

## stress-fio
- `stress-ng --cpu 4 --timeout 30s`
- `stress-ng --vm 2 --vm-bytes 1G --timeout 60s`
- `stress-ng --hdd 1 --hdd-bytes 4G --timeout 60s`
- `fio --name=randread --ioengine=libaio --rw=randread --bs=4k --size=1G --numjobs=4 --runtime=30 --direct=1`
- `fio --name=randwrite --rw=randwrite --bs=4k --size=512M`
- `dd if=/dev/zero of=/tmp/file bs=1M count=1024 oflag=direct`

## iperf3
- `iperf3 -s` — server side
- `iperf3 -c host` — client
- `iperf3 -c host -t 30` — duration
- `iperf3 -c host -P 4` — parallel streams
- `iperf3 -c host -u -b 100M` — UDP at 100Mbps
- `iperf3 -c host -R` — reverse (server sends)

## traceroute-mtr
- `traceroute example.com`
- `traceroute -T -p 443 example.com` — TCP probe
- `traceroute -I example.com` — ICMP
- `mtr example.com` — combined ping+traceroute
- `mtr -r -c 100 example.com` — report mode
- `mtr -T -P 443 example.com` — TCP

## tail-head-less
- `tail -n 100 file`
- `tail -f file` — follow
- `tail -F file` — follow, recreate on rotate
- `tail -f file | grep PAT` — filter live
- `head -n 100 file`
- `head -c 1000 file` — first 1000 bytes
- `less +F file` — like tail -f
- `less +G file` — open at end

## file-disk-tools
- `file binary` — identify type
- `stat file` — metadata
- `du -h file`
- `wc -l file` — count lines
- `wc -w file` — words
- `wc -c file` — bytes
- `md5sum file`
- `sha256sum file`
- `hexdump -C file | head`
- `xxd file | head`

## time-cmd
- `time cmd` — wall + user + sys time
- `/usr/bin/time -v cmd` — detailed (peak RAM, etc)
- `hyperfine 'cmd-a' 'cmd-b'` — benchmark + compare
- `hyperfine --warmup 3 'cmd'`
- `hyperfine -i 'cmd'` — ignore failures

## process-control
- `nohup cmd &` — survive logout
- `disown` — detach from shell
- `bg` — background a stopped (Ctrl-Z) job
- `fg` — foreground
- `jobs` — list shell jobs
- `wait` — wait for all bg
- `wait PID`
- `nice -n 10 cmd` — lower priority
- `renice 10 -p PID`
- `taskset -c 0,1 cmd` — pin to CPUs 0,1
- `ionice -c 3 cmd` — idle IO class

## cron
- `crontab -l` — list
- `crontab -e` — edit
- `crontab -r` — REMOVE all (be careful)
- Format: `m h dom mon dow cmd`
- `*/5 * * * *` — every 5 min
- `0 */2 * * *` — every 2 hours
- `0 3 * * *` — daily at 03:00
- `0 0 * * 0` — weekly Sunday midnight
- `@reboot cmd` — on boot
- Log: `grep CRON /var/log/syslog`

## systemd-timers
- `systemctl list-timers`
- Unit: `[Timer]\nOnCalendar=daily\nUnit=mytask.service`
- `OnCalendar=*-*-* 03:00:00`
- `OnBootSec=5min`
- `Persistent=true` — catch up missed runs

## containerd-crictl
- `crictl ps` — like docker ps for containerd
- `crictl pods` — list pods
- `crictl logs CONTAINER`
- `crictl exec -it CONTAINER bash`
- `crictl images`
- `crictl rmi unused` — remove unused

## podman
- `podman run --rm -it alpine sh`
- `podman ps -a`
- `podman images`
- `podman build -t name .`
- `podman pod create --name mypod`
- `podman pod ps`
- `podman generate kube pod-name > pod.yml` — convert to K8s YAML

## envsubst
- `envsubst < template.yml > output.yml` — substitute env vars
- `VAR=value envsubst < template.yml`

## base64-uuid
- `echo -n 'plain' | base64`
- `echo 'encoded' | base64 -d`
- `uuidgen` — generate UUID v4
- `cat /proc/sys/kernel/random/uuid`
- `openssl rand -hex 16`
- `openssl rand -base64 32`

## dd
- `dd if=src of=dst bs=1M status=progress`
- `dd if=/dev/zero of=/tmp/big bs=1M count=1024`
- `dd if=/dev/sda of=disk.img bs=4M conv=sparse status=progress` — disk image
- `dd if=disk.iso of=/dev/sdX bs=4M status=progress` — burn USB (DANGER)
- `dd if=/dev/urandom of=/tmp/random bs=1M count=100`

## locale-tz
- `locale` — current settings
- `locale -a` — available
- `update-locale LANG=en_US.UTF-8`
- `timedatectl status`
- `timedatectl set-timezone Europe/Moscow`
- `tzselect` — interactive

## date-arithmetic
- `date` — now
- `date -u` — UTC
- `date +%s` — unix timestamp
- `date -d '@1700000000'` — from timestamp
- `date -d 'now - 1 hour'`
- `date -d 'next monday'`
- `date -d 'yesterday' +%F` — ISO date

## chrony-ntp
- `chronyc tracking` — status
- `chronyc sources` — peers
- `chronyc sources -v` — verbose
- `chronyc makestep` — step clock immediately
- `chronyc burst 4/4` — quick re-sync

## kubectl-advanced
- `kubectl get pod -o jsonpath='{.status.phase}'`
- `kubectl get pods -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.status.phase}{"\n"}{end}'`
- `kubectl get pods --field-selector=status.phase!=Running`
- `kubectl scale deployment/X --replicas=5`
- `kubectl autoscale deployment/X --min=2 --max=10 --cpu-percent=70`
- `kubectl drain node-name --ignore-daemonsets --delete-emptydir-data`
- `kubectl cordon node-name`
- `kubectl uncordon node-name`
- `kubectl taint nodes node-name special=true:NoSchedule`
- `kubectl label node node-name disktype=ssd`
- `kubectl annotate pod X key=val`

## kubectl-secrets
- `kubectl create secret generic db-creds --from-literal=user=foo --from-literal=pass=bar`
- `kubectl create secret tls my-tls --cert=cert.pem --key=key.pem`
- `kubectl create secret docker-registry regcred --docker-server=R --docker-username=U --docker-password=P`
- `kubectl get secret X -o jsonpath='{.data.password}' | base64 -d`

## stern
- `stern pod-prefix` — tail logs across pods
- `stern -l app=api -n prod` — by label + namespace
- `stern --since 5m pod-prefix`
- `stern --tail 50 pod-prefix`
- `stern --color always pod-prefix`

## k9s
- `k9s` — interactive K8s TUI
- `:pods` — pods view
- `:svc` — services
- `/pattern` — filter
- `l` — logs (on pod)
- `s` — shell (on pod)
- `d` — describe
- `Ctrl-D` — delete
- `?` — help

## kubectx-kubens
- `kubectx` — list contexts
- `kubectx CONTEXT` — switch
- `kubectx -` — previous
- `kubectx -d CONTEXT` — delete
- `kubens` — list namespaces
- `kubens NAMESPACE` — switch default

## helmfile
- `helmfile sync` — apply all releases
- `helmfile diff` — preview
- `helmfile destroy`
- `helmfile -e ENV sync` — environment-specific
- helmfile.yaml composes multiple helm releases

## kustomize
- `kustomize build .` — render
- `kustomize build overlays/prod` — env-specific
- `kubectl apply -k overlays/prod` — built-in
- `kustomize edit set image NAME=name:newtag`
- `kustomize edit add resource other.yaml`

## kubeval-kubelinter
- `kubeval manifest.yaml` — schema validation
- `kube-linter lint manifest.yaml` — best practices
- `kube-score score manifest.yaml`
- `polaris audit --audit-path manifest.yaml`

## conftest-opa
- `conftest test manifest.yaml` — Rego policies
- `conftest verify` — test the policies
- `opa test policy/`
- `opa eval -d policy/ -i input.json 'data.policy.deny'`

## trivy
- `trivy image alpine:3.18`
- `trivy image --severity HIGH,CRITICAL image`
- `trivy fs .` — scan local dir
- `trivy config .` — scan IaC
- `trivy k8s --report=summary cluster`
- `trivy sbom --format spdx-json image`

## syft-grype
- `syft alpine:3.18` — SBOM
- `syft -o spdx-json image > sbom.json`
- `grype alpine:3.18` — CVEs
- `grype sbom:./sbom.json`

## cosign
- `cosign sign image:tag` — sign image
- `cosign verify image:tag --key cosign.pub`
- `cosign sign-blob file.txt --key cosign.key`
- `cosign verify-blob file.txt --signature sig --key cosign.pub`
- `cosign generate-key-pair`

## act
- `act` — run github actions locally
- `act -l` — list workflows
- `act push` — simulate push event
- `act -j job-name` — single job

## minikube-kind
- `minikube start --driver=docker --kubernetes-version=1.30.0`
- `minikube ip`
- `minikube ssh`
- `minikube dashboard`
- `minikube addons enable ingress`
- `kind create cluster --config kind.yaml`
- `kind delete cluster`
- `kind load docker-image my-image:tag`

## envoy-admin
- `curl localhost:9901/clusters` — cluster status
- `curl localhost:9901/listeners`
- `curl localhost:9901/stats` — all stats
- `curl localhost:9901/server_info`
- `curl localhost:9901/ready`
- `curl -X POST localhost:9901/quitquitquit` — graceful shutdown

## nginx-control
- `nginx -t` — test config
- `nginx -s reload` — graceful reload
- `nginx -s stop` — fast shutdown
- `nginx -s quit` — graceful
- `nginx -V` — version + compile options
- `nginx -T` — dump active config

## haproxy-control
- `haproxy -c -f /etc/haproxy/haproxy.cfg` — config check
- `systemctl reload haproxy` — hot reload
- `echo 'show stat' | socat /var/run/haproxy.sock -` — stats via socket
- `echo 'disable server backend/server' | socat /var/run/haproxy.sock -`
- `echo 'enable server backend/server' | socat /var/run/haproxy.sock -`

## redis-mgmt
- `redis-cli ACL LIST`
- `redis-cli CLUSTER INFO`
- `redis-cli CLUSTER NODES`
- `redis-cli CLUSTER FORGET NODE_ID`
- `redis-cli CLUSTER RESET` — DANGER
- `redis-cli CLIENT LIST`
- `redis-cli CLIENT KILL ADDR ip:port`
- `redis-cli BGSAVE` — async snapshot
- `redis-cli LASTSAVE`

## pg-maintenance
- `pg_basebackup -h primary -U replicator -D /var/lib/postgresql -R` — initial replica
- `pg_dump -h host -U user -Fc db > backup.dump` — custom format
- `pg_restore -h host -U user -d db backup.dump`
- `pg_dumpall -g > globals.sql` — roles + tablespaces
- `pg_ctl reload` — re-read config
- `pg_ctl promote` — promote standby

## mysql-maintenance
- `mysqldump -h host -u user -p db > backup.sql`
- `mysqldump --single-transaction --routines --triggers db > backup.sql`
- `mysql -h host -u user -p db < backup.sql` — restore
- `mysqlcheck --analyze --all-databases`
- `mysqlcheck --optimize --all-databases`
- `xtrabackup --backup --target-dir=/backup` — hot backup

## etcd-cli
- `etcdctl member list`
- `etcdctl endpoint health`
- `etcdctl endpoint status -w table`
- `etcdctl snapshot save /backup/snap.db`
- `etcdctl snapshot restore /backup/snap.db --data-dir=/var/lib/etcd-new`
- `etcdctl alarm list`
- `etcdctl defrag` — reclaim space
- `etcdctl compact REVISION` — old history

## vault-cli
- `vault login -method=oidc`
- `vault status`
- `vault kv get secret/path`
- `vault kv put secret/path key=val`
- `vault kv list secret/`
- `vault kv delete secret/path`
- `vault token revoke TOKEN`
- `vault read pki/issue/role`
- `vault audit list`

## ip-iproute2
- `ip addr show` (or `ip a`)
- `ip route show` (or `ip r`)
- `ip route get 8.8.8.8`
- `ip link show`
- `ip link set eth0 up/down`
- `ip addr add 192.168.1.10/24 dev eth0`
- `ip route add 10.0.0.0/8 via 192.168.1.1`
- `ip neigh show` — ARP table
- `ip -s link` — interface stats

## conntrack
- `conntrack -L` — list
- `conntrack -L -p tcp --dport 443`
- `conntrack -D -s 1.2.3.4` — delete entries
- `conntrack -F` — flush ALL (DANGER)
- `conntrack -E` — stream events
- `cat /proc/sys/net/netfilter/nf_conntrack_count`
- `cat /proc/sys/net/netfilter/nf_conntrack_max`

## tc-traffic-control
- `tc qdisc show`
- `tc qdisc add dev eth0 root netem delay 100ms` — add 100ms latency
- `tc qdisc add dev eth0 root netem loss 5%` — add 5% loss
- `tc qdisc del dev eth0 root` — remove
- `tc -s qdisc` — stats

## sysctl-tuning
- `sysctl -a | grep net.ipv4` — view
- `sysctl -w net.ipv4.tcp_max_syn_backlog=65535` — runtime
- `echo 'net.ipv4.tcp_max_syn_backlog=65535' >> /etc/sysctl.d/99-tune.conf` — persist
- `sysctl --system` — reload all .d files

## ulimit-systemd
- `ulimit -a` — current limits
- `ulimit -n 65535` — open files for shell
- `/etc/security/limits.conf`: `* soft nofile 65535`
- In systemd unit: `LimitNOFILE=65535`

## bcc-tools-popular
- `tcpconnect-bpfcc` — log TCP connect calls
- `tcpaccept-bpfcc` — log accepts
- `tcpretrans-bpfcc` — log retransmits
- `bitesize-bpfcc` — block IO size histogram
- `cachestat-bpfcc` — page cache hit/miss
- `runqlat-bpfcc` — scheduler runqueue latency
- `profile-bpfcc` — CPU sampling profiler

## go-tools
- `go build` — compile
- `go run main.go`
- `go test ./...` — all packages
- `go test -race`
- `go test -bench=. -benchmem`
- `go test -cover`
- `go vet ./...`
- `go mod tidy`
- `go mod why package`
- `go env GOPATH GOCACHE`
- `gofmt -w .`
- `golangci-lint run`

## rust-tools
- `cargo build`
- `cargo build --release`
- `cargo test`
- `cargo run -- arg1 arg2`
- `cargo check` — type-check, no codegen (fast)
- `cargo clippy -- -D warnings`
- `cargo fmt`
- `cargo doc --open`
- `cargo bench`
- `cargo update` — refresh deps
- `cargo tree` — deps graph
- `cargo audit` — CVE check

## python-pip-tools
- `python -m venv .venv`
- `source .venv/bin/activate` (Linux/macOS)
- `pip install -r requirements.txt`
- `pip freeze > requirements.txt`
- `pip install -e .` — editable install
- `pip install --upgrade pip`
- `python -m pytest`
- `pytest -k pattern` — run matching tests
- `pytest -x` — stop on first fail
- `pytest --cov=mypackage`
- `python -m http.server 8000` — quick file server

## npm-yarn
- `npm install` — install all
- `npm install pkg --save`
- `npm install -D pkg` — devDep
- `npm uninstall pkg`
- `npm outdated`
- `npm update`
- `npm audit fix`
- `npm run script-name`
- `npm ci` — clean install from lock
- `yarn` — alternative
- `pnpm install`
- `npx pkg` — run without install