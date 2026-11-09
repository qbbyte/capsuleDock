# 发布前门槛：单元测试、安装器测试和更新脚本测试全部通过才允许打包发布。
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
Push-Location $projectRoot
try {
    $compiler = & (Join-Path $PSScriptRoot 'find-nsis.ps1')
    cargo test --locked --all-targets --features record -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw '自动化测试未通过' }
    & (Join-Path $projectRoot 'tests\native_installer.ps1') -NsisCompiler $compiler
    if ($LASTEXITCODE -ne 0) { throw '安装器测试未通过' }
    & (Join-Path $projectRoot 'tests\update_installer.ps1')
    if ($LASTEXITCODE -ne 0) { throw '更新脚本测试未通过' }
    Write-Output '发布前检查通过：单元测试、安装器测试、更新脚本测试。'
} finally { Pop-Location }
