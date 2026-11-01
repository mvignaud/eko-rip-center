<?php
// E-KO RIP Center : réception du formulaire « Demande de pilote » (/compatibilite/).
// Seul script du site. Il envoie la demande par e-mail à l'équipe, ne garde rien sur le disque
// (hors un compteur par adresse IP hachée, contre les envois en rafale) et ne renvoie que du JSON,
// ou une redirection quand le navigateur poste le formulaire sans JavaScript.

declare(strict_types=1);

const DESTINATAIRE = 'contact@e-ko.rip';
const EXPEDITEUR = 'contact@e-ko.rip';
const PLAFOND_PAR_HEURE = 5;

header('X-Content-Type-Options: nosniff');
header('Cache-Control: no-store');

$json = str_contains($_SERVER['HTTP_ACCEPT'] ?? '', 'application/json');

function repondre(bool $ok, string $message, int $code, bool $json): never
{
    http_response_code($code);
    if ($json) {
        header('Content-Type: application/json; charset=utf-8');
        echo json_encode(['ok' => $ok, 'message' => $message], JSON_UNESCAPED_UNICODE);
    } else {
        // Sans JavaScript : l'ancre affiche le bon message par :target, en CSS seul (revue Codex).
        header('Location: /compatibilite/#' . ($ok ? 'demande-envoyee' : 'demande-erreur'), true, 303);
    }
    exit;
}

if (($_SERVER['REQUEST_METHOD'] ?? '') !== 'POST') {
    header('Allow: POST');
    repondre(false, 'Méthode non autorisée.', 405, true);
}

// Le formulaire ne vient que du site lui-même.
$origine = $_SERVER['HTTP_ORIGIN'] ?? '';
if ($origine !== '' && $origine !== 'https://e-ko.rip') {
    repondre(false, 'Origine refusée.', 403, $json);
}

/** Une ligne : sans retour à la ligne ni caractère de contrôle, coupée à $max caractères. */
function ligne(string $nom, int $max): string
{
    $v = $_POST[$nom] ?? '';
    $v = is_string($v) ? $v : '';
    $v = preg_replace('/[\x00-\x1F\x7F]+/u', ' ', $v) ?? '';
    return mb_substr(trim($v), 0, $max);
}

function texte(string $nom, int $max): string
{
    $v = $_POST[$nom] ?? '';
    $v = is_string($v) ? $v : '';
    $v = preg_replace('/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]+/u', '', str_replace("\r\n", "\n", $v)) ?? '';
    return mb_substr(trim($v), 0, $max);
}

// Pot de miel : un robot remplit le champ caché. On fait comme si tout allait bien.
if (ligne('website', 200) !== '') {
    repondre(true, 'Merci, votre demande est envoyée.', 200, $json);
}

$choix = fn (string $nom, array $permis, string $defaut) => in_array($_POST[$nom] ?? '', $permis, true) ? $_POST[$nom] : $defaut;
$libelles = [
    'impression' => 'Impression', 'decoupe' => 'Découpe', 'impression-decoupe' => 'Impression et découpe',
    'reseau' => 'Réseau (Ethernet)', 'usb' => 'USB', 'serie' => 'Série (RS-232)', 'autre' => 'Autre',
];

$d = [
    'marque' => ligne('marque', 80),
    'modele' => ligne('modele', 120),
    'type' => $choix('type', ['impression', 'decoupe', 'impression-decoupe', 'autre'], 'autre'),
    'branchement' => $choix('branchement', ['reseau', 'usb', 'serie', 'autre'], 'autre'),
    'logiciel' => ligne('logiciel', 120),
    'nombre' => max(1, min(999, (int) ligne('nombre', 4))),
    'capture' => ($_POST['capture'] ?? '') === 'oui',
    'nom' => ligne('nom', 100),
    'societe' => ligne('societe', 120),
    'email' => ligne('email', 160),
    'telephone' => ligne('telephone', 30),
    'message' => texte('message', 3000),
];

if ($d['marque'] === '' || $d['modele'] === '' || $d['nom'] === '') {
    repondre(false, 'Indiquez au moins la marque, le modèle et votre nom.', 422, $json);
}
if (filter_var($d['email'], FILTER_VALIDATE_EMAIL) === false) {
    repondre(false, 'L\'adresse e-mail ne semble pas valide.', 422, $json);
}

// Plafond par adresse IP (hachée, jamais gardée en clair), fenêtre d'une heure.
$cle = hash('sha256', ($_SERVER['REMOTE_ADDR'] ?? '') . '|e-ko.rip|' . gmdate('YmdH'));
$prefixe = sys_get_temp_dir() . '/eko-rip-demande-';
// Les compteurs de plus d'une heure sont effacés à chaque demande (revue Codex).
foreach (glob($prefixe . '*') ?: [] as $vieux) {
    if (@filemtime($vieux) < time() - 3600) {
        @unlink($vieux);
    }
}
// Lecture, contrôle et écriture sous un même verrou : deux envois simultanés ne
// passent pas tous les deux sous le plafond (revue Codex).
$f = @fopen($prefixe . $cle, 'c+');
if ($f === false || !flock($f, LOCK_EX)) {
    repondre(false, 'Le service est momentanément indisponible. Écrivez-nous à ' . DESTINATAIRE . '.', 503, $json);
}
$deja = (int) stream_get_contents($f);
if ($deja >= PLAFOND_PAR_HEURE) {
    flock($f, LOCK_UN);
    fclose($f);
    repondre(false, 'Trop de demandes depuis cette connexion. Réessayez dans une heure, ou écrivez à ' . DESTINATAIRE . '.', 429, $json);
}
ftruncate($f, 0);
rewind($f);
fwrite($f, (string) ($deja + 1));
fflush($f);
flock($f, LOCK_UN);
fclose($f);

$lib = fn (string $v) => $libelles[$v] ?? 'Autre';
$corps = implode("\n", [
    'Nouvelle demande de pilote depuis https://e-ko.rip/compatibilite/',
    '',
    'MACHINE',
    'Marque : ' . $d['marque'],
    'Modèle : ' . $d['modele'],
    'Type : ' . $lib($d['type']),
    'Branchement : ' . $lib($d['branchement']),
    'Logiciel actuel : ' . ($d['logiciel'] ?: '-'),
    'Nombre de machines : ' . $d['nombre'],
    'Capture ou essai possible : ' . ($d['capture'] ? 'oui' : 'non'),
    '',
    'CONTACT',
    'Nom : ' . $d['nom'],
    'Société : ' . ($d['societe'] ?: '-'),
    'E-mail : ' . $d['email'],
    'Téléphone : ' . ($d['telephone'] ?: '-'),
    '',
    'PRÉCISIONS',
    $d['message'] ?: '-',
    '',
    '-- ',
    'Envoyé le ' . gmdate('Y-m-d H:i') . ' UTC. Répondre à ce message écrit directement au demandeur.',
]);

$sujet = mb_encode_mimeheader('Demande de pilote : ' . $d['marque'] . ' ' . $d['modele'], 'UTF-8', 'Q', "\r\n");
$entetes = implode("\r\n", [
    'From: ' . mb_encode_mimeheader('Site E-KO RIP', 'UTF-8', 'Q') . ' <' . EXPEDITEUR . '>',
    'Reply-To: ' . $d['email'],
    'MIME-Version: 1.0',
    'Content-Type: text/plain; charset=UTF-8',
    'Content-Transfer-Encoding: 8bit',
    'X-Mailer: e-ko.rip',
]);

$envoye = mail(DESTINATAIRE, $sujet, $corps, $entetes, '-f' . EXPEDITEUR);
if (!$envoye) {
    error_log('e-ko.rip : échec de mail() pour une demande de pilote');
    repondre(false, 'L\'envoi a échoué. Écrivez-nous directement à ' . DESTINATAIRE . '.', 500, $json);
}

repondre(true, 'Merci, votre demande est envoyée. Nous vous répondons par e-mail.', 200, $json);
