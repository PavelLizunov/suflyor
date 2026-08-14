# Agent-agnostic post-release cleanup. Preview is the default; pass -Apply only
# after reading the plan. Stable releases and draft releases are untouched.
param(
    [switch]$Apply,
    [ValidateRange(1, 20)]
    [int]$KeepPrerelease = 1
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
$mode = if ($Apply) { 'apply' } else { 'preview' }
$repo = (& gh.exe repo view --json nameWithOwner --jq .nameWithOwner).Trim()
if ($LASTEXITCODE -ne 0 -or -not $repo) { throw 'gh repository/auth lookup failed' }

function Say([string]$Action, [string]$Target) {
    Write-Host "[release-cleanup:$mode] $Action $Target"
}

function Assert-LastExit([string]$Operation) {
    if ($LASTEXITCODE -ne 0) { throw "$Operation failed" }
}

function Git-IsAncestor([string]$Commit) {
    if ($Commit -notmatch '^[0-9a-f]{40}$') { return $false }
    & git.exe merge-base --is-ancestor $Commit origin/master 2>$null
    return $LASTEXITCODE -eq 0
}

function Read-Worktrees {
    $items = [System.Collections.Generic.List[object]]::new()
    $current = @{}
    foreach ($line in @(& git.exe worktree list --porcelain)) {
        if (-not $line) {
            if ($current.path) { $items.Add([pscustomobject]$current) }
            $current = @{}
            continue
        }
        if ($line -match '^worktree (.+)$') { $current.path = $Matches[1] }
        elseif ($line -match '^HEAD ([0-9a-f]{40})$') { $current.head = $Matches[1] }
        elseif ($line -match '^branch refs/heads/(.+)$') { $current.branch = $Matches[1] }
        elseif ($line -eq 'detached') { $current.detached = $true }
    }
    if ($current.path) { $items.Add([pscustomobject]$current) }
    return $items
}

function Remove-RebuildableTargets([string]$WorktreePath) {
    $worktreeFull = [IO.Path]::GetFullPath($WorktreePath).TrimEnd('\')
    $candidates = [System.Collections.Generic.List[string]]::new()
    foreach ($crate in @('slint-experiment', 'overlay-backend', 'suflyor-tts', 'suflyor-teratts', 'suflyor-wsola')) {
        $candidates.Add((Join-Path $worktreeFull "$crate\target"))
    }
    foreach ($dir in @(Get-ChildItem -LiteralPath $worktreeFull -Directory -ErrorAction SilentlyContinue)) {
        if ($dir.Name -match '^target([-.].+)?$' -or $dir.Name -match '.+-target$') {
            $candidates.Add($dir.FullName)
        }
    }
    foreach ($candidate in @($candidates | Sort-Object -Unique)) {
        if (-not (Test-Path -LiteralPath $candidate -PathType Container)) { continue }
        $full = [IO.Path]::GetFullPath($candidate)
        if (-not $full.StartsWith("$worktreeFull\", [StringComparison]::OrdinalIgnoreCase)) {
            Say 'KEEP out-of-bounds target' $full
            continue
        }
        $item = Get-Item -LiteralPath $full -Force
        if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
            Say 'KEEP reparse-point target' $full
            continue
        }
        Say 'DELETE rebuildable target' $full
        if ($Apply) { Remove-Item -LiteralPath $full -Recurse -Force }
    }
}

& gh.exe auth status *> $null
if ($LASTEXITCODE -ne 0) { throw 'gh is not authenticated' }
& git.exe fetch origin --prune
if ($LASTEXITCODE -ne 0) { throw 'git fetch failed' }

$deleteBranchOnMerge = (& gh.exe api "repos/$repo" --jq '.delete_branch_on_merge').Trim()
Assert-LastExit 'GitHub repository settings lookup'
if ($deleteBranchOnMerge -ne 'true') {
    Say 'ENABLE GitHub delete_branch_on_merge' $repo
    if ($Apply) {
        & gh.exe api --method PATCH "repos/$repo" -F delete_branch_on_merge=true *> $null
        Assert-LastExit 'enabling delete_branch_on_merge'
    }
}

# Merge only ready, non-draft PRs whose required gate is already green and whose
# head SHA has not moved. Close only a PR whose exact head is already in master.
$openPrs = @((& gh.exe pr list --repo $repo --state open --limit 200 --json number,isDraft,headRefOid,headRefName | ConvertFrom-Json) |
    Where-Object { $_ -and $_.headRefOid })
foreach ($pr in $openPrs) {
    if (Git-IsAncestor $pr.headRefOid) {
        Say 'CLOSE redundant PR' "#$($pr.number) $($pr.headRefName)"
        if ($Apply) {
            & gh.exe pr close $pr.number --repo $repo --comment 'Head commit is already contained in master; closed by post-release cleanup.'
            Assert-LastExit "closing PR #$($pr.number)"
        }
        continue
    }
    $view = & gh.exe pr view $pr.number --repo $repo --json isDraft,mergeable,headRefOid,statusCheckRollup | ConvertFrom-Json
    $gate = @($view.statusCheckRollup | Where-Object {
        $_.name -eq 'gate' -and $_.status -eq 'COMPLETED' -and $_.conclusion -eq 'SUCCESS'
    })
    if (-not $view.isDraft -and $view.mergeable -eq 'MERGEABLE' -and $gate.Count -gt 0) {
        Say 'MERGE green PR' "#$($pr.number) $($pr.headRefName)"
        if ($Apply) {
            & gh.exe pr merge $pr.number --repo $repo --merge --match-head-commit $view.headRefOid
            Assert-LastExit "merging PR #$($pr.number)"
        }
    }
    else {
        Say 'KEEP PR (draft, pending, failing, or conflicted)' "#$($pr.number) $($pr.headRefName)"
    }
}

if ($Apply) {
    & git.exe fetch origin --prune
    if ($LASTEXITCODE -ne 0) { throw 'post-merge fetch failed' }
}

# Keep the newest published prerelease(s), regardless of whether an older
# project version used -rc.N or -pre naming. Stable and draft releases stay.
$releases = @((& gh.exe release list --repo $repo --limit 200 --json tagName,isPrerelease,isDraft,publishedAt | ConvertFrom-Json) |
    Where-Object { $_ -and $_.tagName })
$prereleases = @($releases | Where-Object { $_.isPrerelease -and -not $_.isDraft } |
    Sort-Object { [DateTime]$_.publishedAt } -Descending)
for ($i = $KeepPrerelease; $i -lt $prereleases.Count; $i++) {
    $tag = $prereleases[$i].tagName
    Say 'DELETE old prerelease and tag' $tag
    if ($Apply) {
        & gh.exe release delete $tag --repo $repo --cleanup-tag --yes
        Assert-LastExit "deleting release $tag"
    }
}

$allPrs = @((& gh.exe pr list --repo $repo --state all --limit 500 --json number,state,mergedAt,headRefName,headRefOid | ConvertFrom-Json) |
    Where-Object { $_ -and $_.headRefOid })
$mergedExact = @{}
foreach ($pr in $allPrs) {
    if ($pr.mergedAt) { $mergedExact["$($pr.headRefName)|$($pr.headRefOid)"] = $pr.number }
}

# Delete legacy remote branches only with proof: exact ancestry in master or an
# exact head SHA recorded on a merged PR. GitHub's delete_branch_on_merge repo
# setting handles new PR branches automatically.
$remoteRefs = @(& git.exe for-each-ref --format='%(refname)|%(objectname)' refs/remotes/origin/)
foreach ($line in $remoteRefs) {
    if ($line -notmatch '^refs/remotes/origin/(.+)\|([0-9a-f]{40})$') { continue }
    $branch = $Matches[1]
    $sha = $Matches[2]
    if ($branch -in @('HEAD', 'master')) { continue }
    $safe = (Git-IsAncestor $sha) -or $mergedExact.ContainsKey("$branch|$sha")
    if (-not $safe) {
        Say 'KEEP unproven remote branch' $branch
        continue
    }
    Say 'DELETE merged remote branch' $branch
    if ($Apply) {
        & git.exe push --no-verify origin --delete $branch
        Assert-LastExit "deleting remote branch $branch"
    }
}

$builders = @(Get-Process cargo, rustc -ErrorAction SilentlyContinue)
if ($builders.Count -gt 0) {
    Say 'SKIP local cleanup while cargo/rustc is running' (($builders | Select-Object -ExpandProperty Id) -join ',')
    exit 0
}

$worktrees = @(Read-Worktrees)
if ($worktrees.Count -eq 0) { throw 'git returned no worktrees' }
$primary = [IO.Path]::GetFullPath($worktrees[0].path).TrimEnd('\')
$currentRoot = [IO.Path]::GetFullPath($root).TrimEnd('\')
$keptWorktrees = [System.Collections.Generic.List[object]]::new()

foreach ($wt in $worktrees) {
    $path = [IO.Path]::GetFullPath($wt.path).TrimEnd('\')
    if ($path -eq $primary -or $path -eq $currentRoot) {
        Say 'KEEP active/primary worktree' $path
        continue
    }
    if (-not (Test-Path -LiteralPath $path -PathType Container)) {
        Say 'PRUNE missing worktree registration' $path
        continue
    }
    $dirty = @(& git.exe -C $path status --porcelain=v1 --untracked-files=all)
    $branch = if ($wt.branch) { [string]$wt.branch } else { '' }
    $safe = (Git-IsAncestor $wt.head) -or ($branch -and $mergedExact.ContainsKey("$branch|$($wt.head)"))
    if ($safe -and $dirty.Count -eq 0) {
        Say 'REMOVE clean completed worktree' $path
        if ($Apply) {
            & git.exe worktree remove $path
            Assert-LastExit "removing worktree $path"
        }
    }
    else {
        $reason = if ($dirty.Count -gt 0) { 'dirty/untracked' } else { 'unmerged/unproven' }
        Say "KEEP $reason worktree" $path
        $keptWorktrees.Add($wt)
    }
}

# Targets contain no source state. They are removed from every inactive
# worktree that could not itself be removed. Reparse points are always skipped.
foreach ($wt in $keptWorktrees) {
    Remove-RebuildableTargets $wt.path
}

if ($Apply) {
    & git.exe worktree prune
    Assert-LastExit 'pruning worktrees'
}

$checkedOut = @{}
foreach ($wt in @(Read-Worktrees)) {
    if ($wt.branch) { $checkedOut[[string]$wt.branch] = $true }
}
$localRefs = @(& git.exe for-each-ref --format='%(refname:strip=2)|%(objectname)' refs/heads/)
foreach ($line in $localRefs) {
    if ($line -notmatch '^(.+)\|([0-9a-f]{40})$') { continue }
    $branch = $Matches[1]
    $sha = $Matches[2]
    if ($branch -eq 'master' -or $checkedOut.ContainsKey($branch)) { continue }
    $safe = (Git-IsAncestor $sha) -or $mergedExact.ContainsKey("$branch|$sha")
    if (-not $safe) {
        Say 'KEEP unproven local branch' $branch
        continue
    }
    Say 'DELETE completed local branch' $branch
    if ($Apply) {
        & git.exe branch -D -- $branch
        Assert-LastExit "deleting local branch $branch"
    }
}

Write-Host "[release-cleanup:$mode] complete"
exit 0
