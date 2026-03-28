# veil PowerShell hook
# Add this to your PowerShell profile to activate veil automatically

$global:__veil_last_command = ""

function __veil_preexec {
    param($command)
    $global:__veil_last_command = $command
}

function __veil_precmd {
    $exit_code = $LASTEXITCODE
    if ($global:__veil_last_command -ne "") {
        $dir = (Get-Location).Path
        & veil record $global:__veil_last_command $exit_code $dir 2>$null
        $global:__veil_last_command = ""
    }
}

# Hook into PowerShell's prompt function
$global:__veil_original_prompt = $function:prompt

function prompt {
    __veil_precmd
    & $global:__veil_original_prompt
}

Set-PSReadLineOption -HistoryHandler {
    param($command)
    __veil_preexec $command
}