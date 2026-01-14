$ErrorActionPreference = 'Stop'

$packageName = 'claudectx'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

Remove-Item "$toolsDir\claudectx.exe" -Force -ErrorAction SilentlyContinue
