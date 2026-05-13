#!/usr/bin/env pwsh

param(
    [Parameter(Mandatory = $true)]
    [string]$CommitMessageFile
)

$ErrorActionPreference = "Stop"

$subject = Get-Content -LiteralPath $CommitMessageFile -TotalCount 1

if ($subject -cmatch '^Merge ' -or $subject -cmatch '^Revert "') {
    exit 0
}

$pattern = '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert|enforce)(\([a-z0-9-]+\))?: [a-z]'
if ($subject -cnotmatch $pattern) {
    exit 1
}

$description = $subject -creplace '^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert|enforce)(\([a-z0-9-]+\))?: ', ''
if ($description.Length -lt 5) {
    exit 2
}

if ($description.Length -gt 200) {
    exit 3
}

if ($subject -cmatch '\.$') {
    exit 4
}

exit 0
