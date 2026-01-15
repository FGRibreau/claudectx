$ErrorActionPreference = 'Stop'

$packageName = 'claudectx'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$url64 = "https://github.com/FGRibreau/claudectx/releases/download/v$version$/claudectx_windows_x86_64.zip"

$packageArgs = @{
  packageName    = $packageName
  unzipLocation  = $toolsDir
  url64bit       = $url64
  checksum64     = '$checksum64$'
  checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs
