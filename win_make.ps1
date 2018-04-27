# Resolving lib path
$LIBP=(Resolve-Path .\lb\).Path

# Prepping the environment
$env:LIB="$($env:LIB)$LIBP;"

Write-Host "LIB @ $LIBP"

# Building target
if ($env:BUILD_MODE -eq "release") {
    cargo build --release
} else {
    cargo build
}