# Winbrat connection and job recovery

Read this before any build, test, installer, or live UI work on Winbrat. The
owner workstation is for source control, orchestration, and artifact transfer;
never run Suflyor or its Rust build there as a fallback.

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
| Peer offline or Tailscale ping fails | Host power, Tailscale, or network path is unavailable | Report the timestamp and wait for the owner to restore the host/Tailscale |
| Tailscale ping works and port 22 is open | SSH path is healthy | Continue over SSH |
| Tailscale ping works, port 22 fails, another Windows service responds | Tailscale and Windows are alive; `sshd` or its port-22 firewall rule is broken | Use the SSH recovery below; do not say “Tailscale is down” |
| Tailscale ping works but all tested TCP services fail | Host firewall, network profile, boot state, or ACL needs owner inspection | Report evidence; do not guess or switch control channels |

A DERP relay is a valid Tailscale path. “Direct connection not established” is
not by itself a Tailscale outage when ping and other TCP services work.

## Recover OpenSSH on Winbrat

Use an already provisioned, non-interactive WinRM path only if it works without
printing or inspecting credentials. Otherwise ask the owner to run the
following in an administrator PowerShell on **Winbrat**:

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

## Forbidden fallback

Do not open RustDesk, JetKVM, RDP, browser control, Computer Use, or any other
screen-control channel merely because SSH failed. Use one only when the owner
explicitly requests that exact channel for the current incident and the target
is verified as Winbrat. Never assume the machine currently in front of the
agent is the test machine.

When blocked, report: first failure time, last successful check, Tailscale
result, tested ports, likely failed layer, and the one owner action required.
