<?php
// E-KO RIP Center : lien de téléchargement direct du client, toujours sur la dernière version.
//   /telecharger/mac      → le .dmg (universel, signé et notarisé) de la dernière release qui en a un
//   /telecharger/windows  → l'installateur Windows (.exe, sinon .msi) de la dernière release qui en a un
// La liste des releases vient de l'API GitHub, gardée 10 minutes dans ~/eko-rip-cache (limite de 60 appels
// par heure sans jeton). Si GitHub ne répond pas, on renvoie vers la page des versions.

declare(strict_types=1);

const DEPOT = 'mvignaud/eko-rip-center';
const CACHE_S = 600;
const MOTIFS = [
    'mac' => ['/_universal\.dmg$/i', '/\.dmg$/i'],
    'windows' => ['/x64-setup\.exe$/i', '/\.exe$/i', '/\.msi$/i'],
];

header('X-Content-Type-Options: nosniff');
header('Cache-Control: no-store');

$os = $_GET['os'] ?? '';
$secours = 'https://github.com/' . DEPOT . '/releases/latest';
if (!isset(MOTIFS[$os])) {
    header('Location: /#installer', true, 302);
    exit;
}

function lire_releases(): ?array
{
    // Cache privé au compte, hors du dossier public (public_html/telecharger → ~/eko-rip-cache),
    // et non dans le /tmp commun du serveur, qu'un autre compte pourrait remplir (revue Graphify).
    $dossier = dirname(__DIR__, 2) . '/eko-rip-cache';
    if (!is_dir($dossier)) {
        @mkdir($dossier, 0700);
    }
    $cache = is_dir($dossier) && is_writable($dossier) && fileowner($dossier) === getmyuid()
        ? $dossier . '/releases.json'
        : null;
    if ($cache === null) {
        return lire_github();
    }
    if (is_file($cache) && time() - filemtime($cache) < CACHE_S) {
        $j = json_decode((string) file_get_contents($cache), true);
        if (is_array($j)) {
            return $j;
        }
    }
    $corps = lire_github_brut();
    $j = $corps === null ? null : json_decode($corps, true);
    if (!is_array($j)) {
        // GitHub muet : on garde l'ancienne copie si elle existe, même périmée.
        $vieux = is_file($cache) ? json_decode((string) file_get_contents($cache), true) : null;
        return is_array($vieux) ? $vieux : null;
    }
    @file_put_contents($cache, $corps, LOCK_EX);
    return $j;
}

function lire_github(): ?array
{
    $corps = lire_github_brut();
    $j = $corps === null ? null : json_decode($corps, true);
    return is_array($j) ? $j : null;
}

function lire_github_brut(): ?string
{
    $url = 'https://api.github.com/repos/' . DEPOT . '/releases?per_page=30';
    $corps = null;
    if (function_exists('curl_init')) {
        $c = curl_init($url);
        curl_setopt_array($c, [
            CURLOPT_RETURNTRANSFER => true,
            CURLOPT_TIMEOUT => 5,
            CURLOPT_USERAGENT => 'e-ko.rip',
            CURLOPT_HTTPHEADER => ['Accept: application/vnd.github+json'],
        ]);
        $r = curl_exec($c);
        if (is_string($r) && curl_getinfo($c, CURLINFO_HTTP_CODE) === 200) {
            $corps = $r;
        }
        curl_close($c);
    } else {
        $ctx = stream_context_create(['http' => ['timeout' => 5, 'header' => "User-Agent: e-ko.rip\r\nAccept: application/vnd.github+json\r\n"]]);
        $r = @file_get_contents($url, false, $ctx);
        $corps = is_string($r) ? $r : null;
    }
    return $corps;
}

$releases = lire_releases() ?? [];
foreach ($releases as $rel) {
    if (!empty($rel['draft']) || !empty($rel['prerelease'])) {
        continue;
    }
    foreach (MOTIFS[$os] as $motif) {
        foreach ($rel['assets'] ?? [] as $a) {
            $lien = (string) ($a['browser_download_url'] ?? '');
            if (preg_match($motif, (string) ($a['name'] ?? '')) && str_starts_with($lien, 'https://github.com/' . DEPOT . '/releases/download/')) {
                header('Location: ' . $lien, true, 302);
                exit;
            }
        }
    }
}
header('Location: ' . $secours, true, 302);
