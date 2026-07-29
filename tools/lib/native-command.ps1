Set-StrictMode -Version Latest

function Invoke-NativeChecked {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true)]
        [string]$FilePath,

        [Parameter()]
        [object[]]$CommandArguments = @(),

        [Parameter(Mandatory = $true)]
        [string]$FailureMessage
    )

    # Windows PowerShell 5.1 wraps a native program's stderr records as
    # NativeCommandError when a caller redirects the script's streams. Cargo
    # writes ordinary progress to stderr, so ErrorActionPreference=Stop would
    # otherwise turn successful builds into false failures. PowerShell 7 does
    # not need this compatibility window, but behaves correctly with it.
    $previousErrorActionPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue'
        & $FilePath @CommandArguments
        $nativeExitCode = $LASTEXITCODE
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
    }

    if ($nativeExitCode -ne 0) {
        throw "$FailureMessage (exit $nativeExitCode)"
    }
}
