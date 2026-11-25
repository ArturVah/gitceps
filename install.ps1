# Installs Gitceps into ~\.gitceps and turns it on for every git repo:
#   irm https://raw.githubusercontent.com/ArturVah/gitceps/main/install.ps1 | iex
# Run again (or `gitceps update`) to update. An update leaves him on or off, as he was.
& {
    $ErrorActionPreference = 'Stop'
    $bin = Join-Path $HOME '.gitceps\bin'
    $gitceps = Join-Path $bin 'gitceps.exe'
    # Where the release files are. Another address serves a mirror or a test.
    $from = if ($env:GITCEPS_RELEASE) { $env:GITCEPS_RELEASE } else { 'https://github.com/ArturVah/gitceps/releases/latest/download' }
    $asset = 'gitceps-windows-x86_64.zip'

    $fresh = -not (Test-Path $gitceps)
    $wasOn = (-not $fresh) -and ((& $gitceps status) -eq 'on')

    Write-Host 'Downloading Gitceps...'
    $tmp = Join-Path ([IO.Path]::GetTempPath()) "gitceps-$PID"
    New-Item -ItemType Directory -Force $tmp | Out-Null
    try {
        Invoke-WebRequest "$from/$asset" -OutFile "$tmp\$asset" -UseBasicParsing
        Invoke-WebRequest "$from/SHA256SUMS" -OutFile "$tmp\SHA256SUMS" -UseBasicParsing
        $sum = (Get-FileHash "$tmp\$asset" -Algorithm SHA256).Hash.ToLower()
        if (-not (Select-String -Path "$tmp\SHA256SUMS" -Pattern "^$sum  $([regex]::Escape($asset))$" -Quiet)) {
            throw "The download doesn't match its checksum, so nothing was installed."
        }
        Expand-Archive "$tmp\$asset" -DestinationPath "$tmp\new"

        New-Item -ItemType Directory -Force $bin | Out-Null
        foreach ($program in 'gitceps.exe', 'gitceps-window.exe') {
            $target = Join-Path $bin $program
            # Windows won't replace a program that's running, but lets it be renamed out of the way.
            Remove-Item "$target.old" -ErrorAction SilentlyContinue
            if (Test-Path $target) { Rename-Item $target "$program.old" }
            Move-Item "$tmp\new\$program" $target
            Remove-Item "$target.old" -ErrorAction SilentlyContinue
        }
    } finally {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }

    $path = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (($path -split ';') -notcontains $bin) {
        [Environment]::SetEnvironmentVariable('Path', ("$path".TrimEnd(';') + ";$bin").TrimStart(';'), 'User')
        Write-Host "Added $bin to your PATH. Open a new terminal to use the gitceps command."
    }

    $version = (& $gitceps --version) -replace '^gitceps', 'Gitceps'
    if ($fresh -or $wasOn) {
        & $gitceps on | Out-Null   # says why itself when it can't
        if ($LASTEXITCODE -ne 0) { throw "$version is installed, but he isn't on." }
        Write-Host "$version is installed and on for every repo. Try a git push."
    } else {
        Write-Host "$version is installed. He's off. To turn him on: gitceps on"
    }
}
