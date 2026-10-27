param(
  [switch]$NonInteractive
)

$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$EnvFile = Join-Path $Repo 'server\.env'
$PostHelperDbUrlFile = 'C:\post-helper-db\database-url.txt'
$PsqlExe = 'C:\post-helper-db\env\Library\bin\psql.exe'
$Schema = 'advisor'
$Port = '8082'

function New-Secret($length) {
  $chars = (48..57) + (65..90) + (97..122)
  -join ($chars | Get-Random -Count $length | ForEach-Object { [char]$_ })
}

$values = [ordered]@{}
if (Test-Path $EnvFile) {
  foreach ($line in Get-Content $EnvFile) {
    if ($line -match '^\s*([A-Z_]+)\s*=\s*(.*?)\s*$') { $values[$Matches[1]] = $Matches[2] }
  }
}

if (-not $values['ANTHROPIC_API_KEY'] -and -not $NonInteractive) {
  $secure = Read-Host -Prompt 'ANTHROPIC_API_KEY' -AsSecureString
  $values['ANTHROPIC_API_KEY'] = [System.Net.NetworkCredential]::new('', $secure).Password
}

if (-not (Test-Path $PostHelperDbUrlFile)) {
  throw "No ${PostHelperDbUrlFile}: set up the post_helper database first"
}
$baseUrl = ((Get-Content $PostHelperDbUrlFile -Raw).Trim() -split '\?')[0]
$values['DATABASE_URL'] = "${baseUrl}?sslmode=disable&search_path=$Schema"
$values['PORT'] = $Port

$env:PGGSSENCMODE = 'disable'
& $PsqlExe $baseUrl -c "CREATE SCHEMA IF NOT EXISTS $Schema" | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'Could not create the database schema' }

$newPassword = $null
if (-not $values['ACCESS_PASSWORD']) {
  $typed = ''
  if (-not $NonInteractive) {
    $secure = Read-Host -Prompt 'ACCESS_PASSWORD (empty; Enter generates a random one)' -AsSecureString
    $typed = [System.Net.NetworkCredential]::new('', $secure).Password
  }
  if (-not $typed) {
    $typed = New-Secret 24
    $newPassword = $typed
  }
  $values['ACCESS_PASSWORD'] = $typed
}

if (-not $values['GITHUB_TOKEN'] -and -not $NonInteractive) {
  $secure = Read-Host -Prompt 'GITHUB_TOKEN (needed for edits from chat; Enter skips it)' -AsSecureString
  $typed = [System.Net.NetworkCredential]::new('', $secure).Password
  if ($typed) { $values['GITHUB_TOKEN'] = $typed }
}

$lines = $values.GetEnumerator() | ForEach-Object { "$($_.Key)=$($_.Value)" }
Set-Content -Path $EnvFile -Value $lines -Encoding ascii
Write-Output "Saved $EnvFile (port $Port, schema $Schema)"

if ($newPassword) {
  Set-Clipboard -Value $newPassword
  Write-Output "New access password generated ($($newPassword.Length) chars) and copied to the clipboard"
}
