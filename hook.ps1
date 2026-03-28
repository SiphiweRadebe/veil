# veil PowerShell hook

$global:__veil_last_command = ""

# Intercept every command via the prompt function
$global:__veil_original_prompt = $function:prompt

function prompt {
    $exit_code = $LASTEXITCODE

    # Record the last command into memoir
    if ($global:__veil_last_command -ne "") {
        $dir = (Get-Location).Path
        & "C:\Users\ra\.veil\veil.exe" record "$global:__veil_last_command" "$exit_code" "$dir" 2>$null
        $global:__veil_last_command = ""
    }

    # Capture the next command from history
    $last = (Get-History -Count 1 -ErrorAction SilentlyContinue)
    if ($last) {
        $global:__veil_last_command = $last.CommandLine
    }

    # Run the original prompt
    & $global:__veil_original_prompt
}