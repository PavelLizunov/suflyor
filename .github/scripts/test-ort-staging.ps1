# Exercise the missing-parent failure behind the ort-sys Windows DLL copy panic.
$ErrorActionPreference = 'Stop'
$prepare = Join-Path $PSScriptRoot 'prepare-ort-staging.ps1'
$fixture = Join-Path ([System.IO.Path]::GetTempPath()) ('suflyor-ort-staging-' + [guid]::NewGuid().ToString('N'))

try {
    foreach ($case in @('fresh', 'cached')) {
        $caseRoot = Join-Path $fixture $case
        [void][System.IO.Directory]::CreateDirectory($caseRoot)
        $sourceDll = Join-Path $caseRoot 'source.dll'
        [System.IO.File]::WriteAllBytes($sourceDll, [byte[]](1, 2, 3, 4))
        foreach ($crate in @('overlay-backend', 'slint-experiment')) {
            $crateRoot = Join-Path $caseRoot $crate
            [void][System.IO.Directory]::CreateDirectory($crateRoot)
            if ($case -eq 'cached') {
                $debugRoot = Join-Path $crateRoot 'target/debug'
                [void][System.IO.Directory]::CreateDirectory($debugRoot)
                [System.IO.File]::WriteAllBytes((Join-Path $debugRoot 'onnxruntime.dll'), [byte[]](9, 8, 7))
                [System.IO.File]::WriteAllText((Join-Path $debugRoot 'unrelated.keep'), 'preserve')
            }
            $missingParent = $false
            try {
                [System.IO.File]::Copy($sourceDll, (Join-Path $crateRoot 'target/debug/examples/onnxruntime.dll'))
            } catch {
                if ($_.Exception -is [System.IO.DirectoryNotFoundException] -or
                    $_.Exception.InnerException -is [System.IO.DirectoryNotFoundException]) {
                    $missingParent = $true
                } else {
                    throw
                }
            }
            if (-not $missingParent) { throw "Invalid $case fixture: expected missing-parent copy failure" }
        }

        & $prepare -RepositoryRoot $caseRoot
        foreach ($crate in @('overlay-backend', 'slint-experiment')) {
            $debugRoot = Join-Path (Join-Path $caseRoot $crate) 'target/debug'
            foreach ($destination in @($debugRoot, (Join-Path $debugRoot 'examples'), (Join-Path $debugRoot 'deps'))) {
                $dll = Join-Path $destination 'onnxruntime.dll'
                if (-not [System.IO.File]::Exists($dll)) {
                    [System.IO.File]::Copy($sourceDll, $dll)
                }
                if (-not [System.IO.File]::Exists($dll)) { throw "DLL staging failed for $case" }
            }
        }

        & $prepare -RepositoryRoot $caseRoot
        foreach ($crate in @('overlay-backend', 'slint-experiment')) {
            $debugRoot = Join-Path (Join-Path $caseRoot $crate) 'target/debug'
            $actual = [Convert]::ToBase64String([System.IO.File]::ReadAllBytes((Join-Path $debugRoot 'onnxruntime.dll')))
            $expected = if ($case -eq 'cached') { 'CQgH' } else { 'AQIDBA==' }
            if ($actual -ne $expected) { throw "Existing DLL changed for $case" }
            if ($case -eq 'cached' -and [System.IO.File]::ReadAllText((Join-Path $debugRoot 'unrelated.keep')) -ne 'preserve') {
                throw 'Unrelated cached file changed'
            }
        }
    }
    Write-Output 'ORT staging regression passed: fresh/cached DLL destinations ready; existing files preserved.'
} finally {
    if (Test-Path -LiteralPath $fixture) { Remove-Item -LiteralPath $fixture -Recurse }
}
