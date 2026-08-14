# Winbrat connection and job recovery

Read this before any build, test, installer, or live UI work on Winbrat.
Winbrat is an agent-managed test VM: the agent owns routine recovery and does
not wait for the owner to operate it. The owner workstation is only for source
control, orchestration, and artifact transfer; never run Suflyor or its Rust
build there as a fallback.

## Before starting a remote job

Record these fields in the local task evidence directory:

- exact commit and tree SHA;
- scheduled task name and remote script path;
- remote log, exit-marker, manifest, and artifact paths;
- original content/path of any fixed scheduled-task wrapper temporarily
  replaced for the job.

Start only one Cargo, installer, or UI action at a time. A scheduled task keeps
running after SSH or the Codex task disconnects, so a lost connection is not a
failed build and never authorizes a duplicate start.

## Diagnose from the owner workstation

Use the MagicDNS name; do not copy an IP into evidence.

```powershell
tailscale status
tailscale ping --c 3 windows-brat
Test-NetConnection windows-brat -Port 22
Test-NetConnection windows-brat -Port 5985
```

Interpret the result before acting:

| Evidence | Conclusion | Next action |
|---|---|---|
| Peer offline or Tailscale ping fails | Host power, Tailscale, or network path is unavailable | Use the available Winbrat/Proxmox management path to restore the test VM; involve the owner only after agent-controlled paths fail |
| Tailscale ping works and port 22 is open | SSH path is healthy | Continue over SSH |
| Tailscale ping works, port 22 fails, WinRM responds | Winbrat is manageable; the failure may be `sshd`, its firewall rule, or only the workstation-to-port-22 path | Continue through WinRM, probe port 22 locally on Winbrat, and repair only the failed layer |
| Tailscale ping works but all tested TCP services fail | Host firewall, network profile, boot state, or ACL needs inspection | Use the verified Winbrat console/Proxmox path and inspect the VM directly |

A DERP relay is a valid Tailscale path. “Direct connection not established” is
not by itself a Tailscale outage when ping and other TCP services work.

## Recover OpenSSH on Winbrat

Use the already provisioned encrypted WinRM credential automatically without
printing or inspecting it. First probe `localhost:22` inside Winbrat: if that
works while the owner workstation cannot reach port 22, `sshd` is healthy and
the fault is in the workstation/Tailscale/ACL path. If the local probe fails,
run the following through WinRM or the verified Winbrat console:

```powershell
Get-Service sshd
Set-Service sshd -StartupType Automatic
Start-Service sshd
Get-NetTCPConnection -LocalPort 22 -State Listen
```

If `sshd` reports `Running` but no listener exists:

```powershell
Restart-Service sshd
Get-NetFirewallRule -Name OpenSSH-Server-In-TCP |
    Select-Object Name, Enabled, Direction, Action
```

Enable the rule only when that command confirms it is disabled. Do not reboot
the machine unless service restart and listener/firewall inspection fail.

```powershell
Enable-NetFirewallRule -Name OpenSSH-Server-In-TCP
```

## Resume the interrupted job

After SSH returns, inspect before restarting anything:

```powershell
Get-ScheduledTask -TaskName '<recorded task>'
Get-ScheduledTaskInfo -TaskName '<recorded task>'
Get-Content -LiteralPath '<recorded exit marker>' -ErrorAction SilentlyContinue
Get-Content -LiteralPath '<recorded log>' -Tail 50
Get-Process cargo,rustc,overlay-host,suflyor-tts,suflyor-teratts `
    -ErrorAction SilentlyContinue
```

- `Running`: keep monitoring the original job.
- exit marker `0`: collect and hash the existing artifacts; do not rebuild.
- non-zero exit: diagnose that logged failure on the same exact tree.
- no marker and no related process: inspect the scheduled task result and log;
  restart only after proving the original job ended.

Restore any temporarily replaced task wrapper after the job completes. Before
publishing, verify the artifact manifest still names the recorded commit/tree.

## Host identity and control channels

Recovery priority is SSH, then WinRM, then a verified Winbrat/Proxmox console.
RustDesk, JetKVM, RDP, browser control, or Computer Use may be used for Winbrat
when a GUI/console is genuinely required; routine service and file recovery
stays in the shell. Before any screen-control action, verify that the target is
Winbrat. Never click, launch Suflyor, build, install, or test on the owner's
workstation under the assumption that it is the test VM.

When all agent-controlled recovery paths fail, report: first failure time, last
successful check, Tailscale result, tested ports, failed recovery paths, likely
failed layer, and the one owner action still required.
