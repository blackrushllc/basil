param(
  [Parameter(Mandatory=$true)][string]$Token,
  [Parameter(Mandatory=$true)][string]$Replacement,
  [string]$Root = "E:\\Projects\\Yore\\basil\\examples"
)

$pattern = "\b" + [regex]::Escape($Token) + "\b"
$files = Get-ChildItem -Path $Root -Recurse -Filter *.basil -File
foreach ($f in $files) {
  $p = $f.FullName
  $s = Get-Content -LiteralPath $p -Raw -Encoding UTF8
  $t = [regex]::Replace($s, $pattern, $Replacement)
  if ($t -ne $s) {
    Set-Content -LiteralPath $p -Value $t -Encoding UTF8
    Write-Host ("Updated {0} -> {1}: {2}" -f $Token, $Replacement, $p)
  }
}