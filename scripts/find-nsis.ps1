# 按显式传入、PATH、NSIS_HOME、默认安装目录的顺序定位 makensis.exe。
param([string]$NsisCompiler)
$ErrorActionPreference = 'Stop'
if ($NsisCompiler) { $candidate = $NsisCompiler }
else {
    $found = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($found) { $candidate = $found.Source }
    elseif ($env:NSIS_HOME) { $candidate = Join-Path $env:NSIS_HOME 'makensis.exe' }
    else { $candidate = Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe' }
}
if (-not (Test-Path -LiteralPath $candidate)) { throw 'NSIS compiler not found; install NSIS or pass -NsisCompiler' }
Write-Output $candidate
