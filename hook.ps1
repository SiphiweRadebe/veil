# veil PowerShell hook

$global:__veil_last_command = ""

$global:__veil_original_prompt = $function:prompt

function prompt {
    $exit_code = $LASTEXITCODE

    if ($global:__veil_last_command -ne "") {
        $dir = (Get-Location).Path
        # Record into memoir
        & "C:\Users\ra\.veil\veil.exe" record "$global:__veil_last_command" "$exit_code" "$dir" 2>$null
        $global:__veil_last_command = ""
    }

    # Capture next command from history
    $last = (Get-History -Count 1 -ErrorAction SilentlyContinue)
    if ($last) {
        $global:__veil_last_command = $last.CommandLine

        # Take a snapshot BEFORE the next command runs
        $dir = (Get-Location).Path
        Start-Job -ScriptBlock {
            & "C:\Users\ra\.veil\veil.exe" snapshot $using:__veil_last_command $using:dir 2>$null
        } | Out-Null
    }

    & $global:__veil_original_prompt
}

# veil go — jump to a bookmarked directory
function veil-go {
    param([string]$name)
    $result = & "C:\Users\ra\.veil\veil.exe" go $name 2>&1
    if ($result -match "^VEIL_CD:(.+)$") {
        $path = $Matches[1]
        Set-Location $path
        Write-Host "  $("→".ToString()) $path" -ForegroundColor DarkGray
    } else {
        Write-Host $result
    }
}

Set-Alias -Name vg -Value veil-go