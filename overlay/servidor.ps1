# Servidor local do overlay de comentaristas (sem Python, so o PowerShell do Windows).
# Serve os arquivos desta pasta em http://127.0.0.1:8777/ e salva a configuracao
# enviada pela pagina config.html no arquivo config.json.
# Acessivel apenas deste computador. Usa o HttpListener do Windows (nao precisa de administrador).

$ErrorActionPreference = 'Stop'
$Porta = 8777
$Raiz  = $PSScriptRoot
$Base  = "http://127.0.0.1:$Porta"
$Host.UI.RawUI.WindowTitle = "Overlay comentaristas - porta $Porta - NAO FECHE durante a live"

$Tipos = @{
  '.html'='text/html; charset=utf-8'; '.htm'='text/html; charset=utf-8'
  '.js'='application/javascript; charset=utf-8'; '.css'='text/css; charset=utf-8'
  '.json'='application/json; charset=utf-8'; '.txt'='text/plain; charset=utf-8'
  '.png'='image/png'; '.jpg'='image/jpeg'; '.jpeg'='image/jpeg'; '.gif'='image/gif'
  '.svg'='image/svg+xml'; '.webp'='image/webp'; '.ico'='image/x-icon'
  '.mp4'='video/mp4'; '.webm'='video/webm'; '.mov'='video/quicktime'
  '.woff'='font/woff'; '.woff2'='font/woff2'; '.ttf'='font/ttf'; '.otf'='font/otf'
}

function Responder-Texto($res, [int]$cod, [string]$tipo, [string]$texto) {
  $b = [System.Text.Encoding]::UTF8.GetBytes($texto)
  $res.StatusCode = $cod
  $res.ContentType = $tipo
  $res.ContentLength64 = $b.Length
  if ($b.Length) { $res.OutputStream.Write($b, 0, $b.Length) }
}

function Tratar($ctx) {
  $req = $ctx.Request
  $res = $ctx.Response
  $res.AddHeader('Cache-Control', 'no-store')
  $caminho = [Uri]::UnescapeDataString($req.Url.AbsolutePath)

  # ---- salvar configuracao (vem da pagina config.html) ----
  if ($caminho -eq '/salvar') {
    if ($req.HttpMethod -ne 'POST') { Responder-Texto $res 405 'text/plain' 'use POST'; return }
    # Protecao: so aceita JSON vindo da propria pagina local (outros sites nao conseguem gravar)
    $origem = $req.Headers['Origin']
    if ($origem -and $origem -ne $Base -and $origem -ne "http://localhost:$Porta") { Responder-Texto $res 403 'text/plain' 'origem nao permitida'; return }
    if ("$($req.ContentType)" -notlike 'application/json*') { Responder-Texto $res 400 'text/plain' 'esperado JSON'; return }
    if ($req.ContentLength64 -gt 1048576) { Responder-Texto $res 413 'text/plain' 'grande demais'; return }
    $leitor = New-Object System.IO.StreamReader($req.InputStream, [System.Text.Encoding]::UTF8)
    $txt = $leitor.ReadToEnd(); $leitor.Close()
    try { $null = $txt | ConvertFrom-Json } catch { Responder-Texto $res 400 'text/plain' 'JSON invalido'; return }
    [System.IO.File]::WriteAllText((Join-Path $Raiz 'config.json'), $txt, (New-Object System.Text.UTF8Encoding $false))
    Responder-Texto $res 200 'application/json; charset=utf-8' '{"ok":true}'
    Write-Host ("  [{0}] configuracao salva" -f (Get-Date -Format 'HH:mm:ss')) -ForegroundColor Green
    return
  }

  if ($req.HttpMethod -ne 'GET' -and $req.HttpMethod -ne 'HEAD') { Responder-Texto $res 405 'text/plain' 'metodo nao suportado'; return }

  # ---- arquivos estaticos ----
  if ($caminho -eq '/') { $caminho = '/config.html' }
  $rel = $caminho.TrimStart('/') -replace '/', '\'
  $full = [System.IO.Path]::GetFullPath((Join-Path $Raiz $rel))
  if (-not $full.StartsWith($Raiz.TrimEnd('\') + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
    Responder-Texto $res 403 'text/plain' 'proibido'; return
  }
  if (-not (Test-Path -LiteralPath $full -PathType Leaf)) {
    Responder-Texto $res 404 'text/plain; charset=utf-8' "Arquivo nao encontrado: $caminho"; return
  }
  $ext = [System.IO.Path]::GetExtension($full).ToLower()
  $tipo = $Tipos[$ext]; if (-not $tipo) { $tipo = 'application/octet-stream' }
  $fs = [System.IO.File]::OpenRead($full)
  try {
    $res.StatusCode = 200
    $res.ContentType = $tipo
    $res.ContentLength64 = $fs.Length
    if ($req.HttpMethod -eq 'GET') { $fs.CopyTo($res.OutputStream) }
  } finally { $fs.Close() }
}

# ---- inicia ----
$listener = New-Object System.Net.HttpListener
$listener.Prefixes.Add("http://127.0.0.1:$Porta/")
$listener.Prefixes.Add("http://localhost:$Porta/")
try {
  $listener.Start()
} catch {
  Write-Host ""
  Write-Host "ERRO: nao foi possivel abrir a porta $Porta." -ForegroundColor Red
  Write-Host "Provavelmente o servidor ja esta aberto em outra janela. Feche a outra janela ou troque `$Porta em servidor.ps1."
  Write-Host "Detalhe: $($_.Exception.Message)"
  Write-Host ""
  Read-Host "Aperte Enter para sair"
  exit 1
}

Write-Host ""
Write-Host "  SERVIDOR DO OVERLAY LIGADO" -ForegroundColor Cyan
Write-Host "  ---------------------------------------------------------------"
Write-Host "  Configurar IDs:   $Base/config.html"
Write-Host "  URL para o OBS:   $Base/comentaristas.html" -ForegroundColor Yellow
Write-Host "                    (fonte Navegador, SEM 'Arquivo local', 1920x1080)"
Write-Host "  ---------------------------------------------------------------"
Write-Host "  Deixe esta janela aberta durante a live. Fechar = cameras param."
Write-Host ""

if ($env:OVERLAY_SEM_NAVEGADOR -ne '1') { Start-Process "$Base/config.html" }

while ($listener.IsListening) {
  $ctx = $listener.GetContext()
  try {
    Tratar $ctx
  } catch {
    # navegador que fecha a conexao no meio (ex.: troca de pagina) nao e problema
    if ($_.Exception.Message -notmatch 'rede|network|conex|connection') {
      Write-Host ("  aviso: {0}" -f $_.Exception.Message) -ForegroundColor DarkYellow
    }
  } finally {
    try { $ctx.Response.Close() } catch {}
  }
}
