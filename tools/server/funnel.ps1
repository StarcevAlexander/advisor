$Tailscale = 'C:\Program Files\Tailscale\tailscale.exe'
$HttpsPort = 8443
$LocalPort = 8081

& $Tailscale funnel --bg --https=$HttpsPort $LocalPort
if ($LASTEXITCODE -ne 0) { throw 'tailscale funnel failed' }
& $Tailscale funnel status
