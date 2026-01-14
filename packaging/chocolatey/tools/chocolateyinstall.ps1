$ErrorActionPreference = 'Stop'

$packageName = 'claudectx'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$url64 = "https://github.com/FGRibreau/claudectx/releases/download/v0.1.0/claudectx_windows_x86_64.zip"

$packageArgs = @{
  packageName    = $packageName
  unzipLocation  = $toolsDir
  url64bit       = $url64
  checksum64     = 'PLACEHOLDER_CHECKSUM_WILL_BE_UPDATED_ON_RELEASE'
  checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs
