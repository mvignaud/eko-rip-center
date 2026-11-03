<?php
// E-KO RIP Center : lien de téléchargement direct du client, toujours sur la dernière version.
//   /telecharger/mac      → le .dmg (universel, signé et notarisé) de la dernière release qui en a un
//   /telecharger/windows  → la page GitHub de la dernière release qui a un installateur Windows (non signé)
//   /telecharger/infos.json → version, taille et SHA-256 des fichiers, pour la page /telecharger/
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
$infos = isset($_GET['infos']);
$secours = 'https://github.com/' . DEPOT . '/releases/latest';
if (!$infos && !isset(MOTIFS[$os])) {
    header('Location: /telecharger/', true, 302);
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

/** Dernière release publiée qui a un fichier pour ce système : [release, fichier] ou null. */
function trouver(array $releases, string $os): ?array
{
    foreach ($releases as $rel) {
        if (!empty($rel['draft']) || !empty($rel['prerelease'])) {
            continue;
        }
        foreach (MOTIFS[$os] as $motif) {
            foreach ($rel['assets'] ?? [] as $a) {
                $lien = (string) ($a['browser_download_url'] ?? '');
                if (preg_match($motif, (string) ($a['name'] ?? '')) && str_starts_with($lien, 'https://github.com/' . DEPOT . '/releases/download/')) {
                    return [$rel, $a];
                }
            }
        }
    }
    return null;
}

$releases = lire_releases() ?? [];

// /telecharger/infos.json : version, taille et empreinte SHA-256 de chaque fichier, affichées
// sur /telecharger/ avant le téléchargement (l'empreinte est celle que GitHub calcule).
if ($infos) {
    header('Content-Type: application/json; charset=utf-8');
    $sortie = [];
    foreach (array_keys(MOTIFS) as $systeme) {
        $t = trouver($releases, $systeme);
        if ($t === null) {
            continue;
        }
        [$rel, $a] = $t;
        $digest = (string) ($a['digest'] ?? '');
        $sortie[$systeme] = [
            'version' => ltrim((string) ($rel['tag_name'] ?? ''), 'v'),
            'date' => substr((string) ($rel['published_at'] ?? ''), 0, 10),
            'fichier' => (string) ($a['name'] ?? ''),
            'taille' => (int) ($a['size'] ?? 0),
            'sha256' => str_starts_with($digest, 'sha256:') ? substr($digest, 7) : null,
            'release' => (string) ($rel['html_url'] ?? $secours),
        ];
    }
    echo json_encode($sortie, JSON_UNESCAPED_SLASHES | JSON_UNESCAPED_UNICODE);
    exit;
}

// Mac : le .dmg, signé et notarisé, part directement. Windows : tant que l'installateur n'est pas
// signé, jamais de .exe en un clic (Google le classe en « pages trompeuses ») : la page de la release,
// qui montre le fichier, sa taille et son empreinte.
$t = trouver($releases, $os);
$cible = $t === null ? $secours : ($os === 'mac' ? (string) $t[1]['browser_download_url'] : (string) ($t[0]['html_url'] ?? $secours));
header('Location: ' . $cible, true, 302);
