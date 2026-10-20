# Exercise the actual workflow's native Git exit-code handling without GitHub mutations.
param([string]$WorkflowPath = (Join-Path $PSScriptRoot '../workflows/docs-after-release.yml'))
$ErrorActionPreference = 'Stop'
$workflow = [IO.File]::ReadAllText($WorkflowPath)
$match = [regex]::Match($workflow, '(?s)        run: \|\r?\n(?<body>          if \(git diff.*?)(?=\r?\n          \$branch = )|        run: \|\r?\n(?<body>          git diff --quiet -- README\.md.*?)(?=\r?\n          \$branch = )')
if (-not $match.Success) { throw 'Cannot find the release documentation no-change guard.' }
$guard = ($match.Groups['body'].Value -replace '(?m)^          ', '')
$temp = Join-Path ([IO.Path]::GetTempPath()) ('suflyor-docs-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory $temp
$runner = Join-Path $temp 'guard.ps1'
[IO.File]::WriteAllText($runner, "$guard`nWrite-Output 'CHANGE_REQUIRED'", [Text.UTF8Encoding]::new($false))
$pwsh = (Get-Process -Id $PID).Path
Push-Location $temp
try {
    git init --quiet
    git config user.name 'Release docs test'
    git config user.email 'release-docs-test@example.invalid'
    [IO.File]::WriteAllText((Join-Path $temp 'README.md'), 'Current release')
    git add README.md
    git commit --quiet -m 'test fixture'
    if ($LASTEXITCODE) { throw 'Cannot initialize test repository.' }

    $output = & $pwsh -NoProfile -File $runner 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0 -or $output -notmatch 'Documentation already matches' -or $output -match 'CHANGE_REQUIRED') {
        throw "Unchanged README did not stop before GitHub mutations: $output"
    }
    Write-Output 'PASS: unchanged README stops.'

    Add-Content README.md 'Next release'
    $output = & $pwsh -NoProfile -File $runner 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0 -or $output -notmatch 'CHANGE_REQUIRED') {
        throw "Changed README did not reach the update path: $output"
    }
    Write-Output 'PASS: changed README continues.'

    $outside = Join-Path $temp 'outside'
    $null = New-Item -ItemType Directory $outside
    Move-Item .git (Join-Path $temp 'git-metadata')
    Push-Location $outside
    try { $output = & $pwsh -NoProfile -File $runner 2>&1 | Out-String; $code = $LASTEXITCODE }
    finally { Pop-Location }
    if ($code -eq 0 -or $output -match 'CHANGE_REQUIRED') { throw 'Git failure did not fail closed.' }
    Write-Output 'PASS: Git failure stops with an error.'
} finally {
    Pop-Location
    Remove-Item -LiteralPath $temp -Recurse -Force
}
# The expected failing child must not become the Actions step exit status.
exit 0
