# Research parser only: never execute or dot-source the input script.
param(
    [Parameter(Mandatory=$true)][string]$SourceFile,
    [Parameter(Mandatory=$true)][string]$SourceLabel
)
$ErrorActionPreference = 'Stop'
if ($PSVersionTable.PSVersion.ToString() -ne '7.4.13') {
    throw 'PowerShell research parser version must be 7.4.13'
}
$utf8 = [System.Text.UTF8Encoding]::new($false, $true)
$sourceBytes = [System.IO.File]::ReadAllBytes($SourceFile)
$source = $utf8.GetString($sourceBytes)
# Canonical ParseFile consumes UTF8 BOM. Match that decoding while keeping raw byte ranges.
$bomBytes = 0
if ($source.Length -gt 0 -and [int]$source[0] -eq 0xFEFF) {
    $source = $source.Substring(1)
    $bomBytes = 3
}
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseInput(
    $source, $SourceLabel, [ref]$tokens, [ref]$parseErrors
)
$sourceHash = [Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData($sourceBytes)).ToLowerInvariant()
$wanted = @('FunctionDefinitionAst', 'TypeDefinitionAst', 'FunctionMemberAst', 'PropertyMemberAst')
$nodes = @($ast.FindAll({ param($node) $node.GetType().Name -in $wanted }, $true))
$rows = [System.Collections.Generic.List[object]]::new()
foreach ($node in $nodes) {
    $kind = $node.GetType().Name
    $name = $node.Name
    $extent = $node.Extent
    $begin = $bomBytes + $utf8.GetByteCount($source.Substring(0, $extent.StartOffset))
    $end = $bomBytes + $utf8.GetByteCount($source.Substring(0, $extent.EndOffset))
    $id = '{0}:{1}:{2}:{3}' -f $SourceLabel, $begin, $end, $kind
    $parents = [System.Collections.Generic.List[string]]::new()
    $scopes = [System.Collections.Generic.List[string]]::new()
    $parent = $node.Parent
    while ($null -ne $parent) {
        if ($parent.GetType().Name -in $wanted) {
            $parentBegin = $bomBytes + $utf8.GetByteCount($source.Substring(0, $parent.Extent.StartOffset))
            $parentEnd = $bomBytes + $utf8.GetByteCount($source.Substring(0, $parent.Extent.EndOffset))
            $parents.Insert(0, ('{0}:{1}:{2}:{3}' -f $SourceLabel, $parentBegin, $parentEnd, $parent.GetType().Name))
            $scopes.Insert(0, $parent.Name)
        }
        $parent = $parent.Parent
    }
    $signatureEnd = $extent.EndOffset
    if ($null -ne $node.PSObject.Properties['Body'] -and $null -ne $node.Body) {
        $signatureEnd = $node.Body.Extent.StartOffset
    } else {
        $newline = $source.IndexOf("`n", $extent.StartOffset)
        if ($newline -ge 0) { $signatureEnd = [Math]::Min($newline, $extent.EndOffset) }
    }
    $signature = $source.Substring($extent.StartOffset, $signatureEnd - $extent.StartOffset).TrimEnd()
    $rows.Add([ordered]@{
        id = $id; path = $SourceLabel; language = 'powershell'; kind = $kind; name = $name
        parent_ids = @($parents.ToArray()); scope_names = @($scopes.ToArray())
        start_byte = $begin; end_byte = $end; start_line = $extent.StartLineNumber; end_line = $extent.EndLineNumber
        signature_source = $signature; attributes = @(); cfg_attributes = @(); test_conditional = $false
        macro_expanded = $false; semantic_acceptance = $false; source_sha256 = $sourceHash
        parser = 'System.Management.Automation.Language.Parser 7.4.13'
    })
}
$errors = @($parseErrors | ForEach-Object {
    [ordered]@{
        kind = 'ParseError'; error_id = $_.ErrorId; incomplete_input = $_.IncompleteInput
        start_byte = $bomBytes + $utf8.GetByteCount($source.Substring(0, $_.Extent.StartOffset))
        end_byte = $bomBytes + $utf8.GetByteCount($source.Substring(0, $_.Extent.EndOffset))
        start_line = $_.Extent.StartLineNumber; end_line = $_.Extent.EndLineNumber
    }
})
[ordered]@{
    declarations = @($rows.ToArray()); parse_errors = $errors; has_parse_error = ($errors.Count -gt 0)
    parser = 'System.Management.Automation.Language.Parser 7.4.13'
    token_count = @($tokens).Count; script_executed = $false
} | ConvertTo-Json -Depth 20 -Compress
