param(
  [string]$Root = "E:\\Projects\\Yore\\basil\\examples"
)

$tokens = @(
  @{ Pattern = "\\bFUNC\\b"; Replacement = 'func' },
  @{ Pattern = "\\bDIM\\b"; Replacement = 'dim' },
  @{ Pattern = "\\bEXIT\\b"; Replacement = 'exit' },
  @{ Pattern = "\\bEACH\\b"; Replacement = 'each' }
)

Get-ChildItem -Path $Root -Recurse -Filter *.basil -File | ForEach-Object {
  $p = $_.FullName
  $orig = Get-Content -LiteralPath $p -Encoding UTF8
  $out = @()
  foreach ($line in $orig) {
    $newLine = $line
    foreach ($t in $tokens) {
      $newLine = [regex]::Replace($newLine, $t.Pattern, $t.Replacement)
    }
    $out += ,$newLine
  }
  if (($out -join "`r`n") -ne ($orig -join "`r`n")) {
    Set-Content -LiteralPath $p -Value $out -Encoding UTF8
    Write-Host "Updated: $p"
  }
}