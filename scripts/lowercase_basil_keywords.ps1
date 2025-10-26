param(
    [string]$Root = "E:\\Projects\\Yore\\basil\\examples"
)

# Keywords to lowercase (standalone tokens only)
$keywords = @(
    'PRINTLN','PRINT','INPUT','IF','THEN','ELSE','END','WHILE','BREAK','CONTINUE',
    'FOR','TO','STEP','NEXT','FUNCTION','RETURN','CLASS','TRUE','FALSE','AND','OR','NOT','LET','BEGIN',
    'FUNC','DIM','EXIT','EACH','IN','AS'
)
$keywordPattern = "\b(?:" + ($keywords -join "|") + ")\b"
$rxKeywords = [System.Text.RegularExpressions.Regex]::new($keywordPattern, [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
$rxREM = [System.Text.RegularExpressions.Regex]::new("\bREM\b", [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
$rxFUNC = [System.Text.RegularExpressions.Regex]::new("\bFUNC\b")
$rxDIM  = [System.Text.RegularExpressions.Regex]::new("\bDIM\b")
$rxEXIT = [System.Text.RegularExpressions.Regex]::new("\bEXIT\b")
$rxEACH = [System.Text.RegularExpressions.Regex]::new("\bEACH\b")
$rxIN   = [System.Text.RegularExpressions.Regex]::new("\bIN\b")
$rxAS   = [System.Text.RegularExpressions.Regex]::new("\bAS\b")

function Process-Code {
    param([string]$code)
    if ([string]::IsNullOrEmpty($code)) { return $code }

    # Find REM as a standalone token (comment) and treat rest of line as comment
    $m = $rxREM.Match($code)
    if ($m.Success) {
        $before = $code.Substring(0, $m.Index)
        $afterStart = $m.Index + $m.Length
        # Targeted replacements for specific tokens first (instance regex)
        $before2 = $rxFUNC.Replace($before, 'func')
        $before2 = $rxDIM.Replace($before2, 'dim')
        $before2 = $rxEXIT.Replace($before2, 'exit')
        $before2 = $rxEACH.Replace($before2, 'each')
        $before2 = $rxIN.Replace($before2, 'in')
        $before2 = $rxAS.Replace($before2, 'as')
        $beforeProcessed = $rxKeywords.Replace($before2, { param($mm) $mm.Value.ToLowerInvariant() })
        return $beforeProcessed + 'rem' + $code.Substring($afterStart)
    }
    else {
        # Targeted replacements for specific tokens first (instance regex)
        $code2 = $rxFUNC.Replace($code, 'func')
        $code2 = $rxDIM.Replace($code2, 'dim')
        $code2 = $rxEXIT.Replace($code2, 'exit')
        $code2 = $rxEACH.Replace($code2, 'each')
        $code2 = $rxIN.Replace($code2, 'in')
        $code2 = $rxAS.Replace($code2, 'as')
        return $rxKeywords.Replace($code2, { param($mm) $mm.Value.ToLowerInvariant() })
    }
}

function Process-Line {
    param([string]$line)
    if ($null -eq $line) { return $line }
    $sb = [System.Text.StringBuilder]::new()
    $len = $line.Length
    $i = 0
    $inStr = $false
    $segmentStart = 0

    while ($i -lt $len) {
        $ch = $line[$i]
        if ($ch -eq '"') {
            if (-not $inStr) {
                # flush code before string
                if ($i -gt $segmentStart) {
                    $code = $line.Substring($segmentStart, $i - $segmentStart)
                    [void]$sb.Append((Process-Code $code))
                }
                $inStr = $true
                [void]$sb.Append($ch)
                $i++
                continue
            }
            else {
                # inside string; handle doubled quote as escaped quote
                if (($i + 1) -lt $len -and $line[$i + 1] -eq '"') {
                    [void]$sb.Append('""')
                    $i += 2
                    continue
                }
                else {
                    $inStr = $false
                    [void]$sb.Append($ch)
                    $i++
                    $segmentStart = $i
                    continue
                }
            }
        }
        # If we're inside a string and it's not a quote, append the character
        if ($inStr) {
            [void]$sb.Append($ch)
            $i++
            continue
        }
        if (-not $inStr -and $ch -eq "'") {
            # Start of apostrophe comment: flush code then append rest unchanged
            if ($i -gt $segmentStart) {
                $code = $line.Substring($segmentStart, $i - $segmentStart)
                [void]$sb.Append((Process-Code $code))
            }
            [void]$sb.Append($line.Substring($i))
            return $sb.ToString()
        }
        $i++
    }

    # End of line
    if (-not $inStr) {
        if ($len -gt $segmentStart) {
            $code = $line.Substring($segmentStart)
            [void]$sb.Append((Process-Code $code))
        }
    }
    else {
        # Unclosed string: append the tail unchanged
        if ($len -gt $segmentStart) {
            [void]$sb.Append($line.Substring($segmentStart))
        }
    }
    return $sb.ToString()
}

# Enumerate only .basil files (avoid Include quirk by using -Filter with -Recurse)
$files = Get-ChildItem -Path $Root -Recurse -Filter *.basil -File
foreach ($f in $files) {
    $orig = Get-Content -LiteralPath $f.FullName -Encoding UTF8
    $outLines = @()
    foreach ($line in $orig) {
        $outLines += ,(Process-Line $line)
    }
    # Only write back if changed
    $origText = ($orig -join "`r`n")
    $newText = ($outLines -join "`r`n")
    if ($origText -ne $newText) {
        Set-Content -LiteralPath $f.FullName -Value $outLines -Encoding UTF8
        Write-Host "Updated: $($f.FullName)"
    }
}
