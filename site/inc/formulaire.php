<?php
// E-KO RIP Center : outils communs aux formulaires du site (demande de pilote, contact).
// Rien n'est gardé sur le disque, hors un compteur par adresse IP hachée contre les envois en rafale.
// Inclus par les scripts ; le dossier inc/ n'est jamais servi (inc/.htaccess).

declare(strict_types=1);

const FORM_DESTINATAIRE = 'contact@e-ko.rip';
const FORM_EXPEDITEUR = 'contact@e-ko.rip';

/** Réponse JSON (avec JavaScript) ou redirection vers $page#$ancre (sans JavaScript, message par :target). */
function form_repondre(bool $ok, string $message, int $code, bool $json, string $page, string $ancre_ok, string $ancre_erreur): never
{
    http_response_code($code);
    if ($json) {
        header('Content-Type: application/json; charset=utf-8');
        echo json_encode(['ok' => $ok, 'message' => $message], JSON_UNESCAPED_UNICODE);
    } else {
        header('Location: ' . $page . '#' . ($ok ? $ancre_ok : $ancre_erreur), true, 303);
    }
    exit;
}

/** Une ligne : sans retour à la ligne ni caractère de contrôle, coupée à $max caractères. */
function form_ligne(string $nom, int $max): string
{
    $v = $_POST[$nom] ?? '';
    $v = is_string($v) ? $v : '';
    $v = preg_replace('/[\x00-\x1F\x7F]+/u', ' ', $v) ?? '';
    return mb_substr(trim($v), 0, $max);
}

/** Un texte sur plusieurs lignes, sans caractère de contrôle, coupé à $max caractères. */
function form_texte(string $nom, int $max): string
{
    $v = $_POST[$nom] ?? '';
    $v = is_string($v) ? $v : '';
    $v = preg_replace('/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]+/u', '', str_replace("\r\n", "\n", $v)) ?? '';
    return mb_substr(trim($v), 0, $max);
}

/** Une valeur parmi $permis, sinon $defaut. */
function form_choix(string $nom, array $permis, string $defaut): string
{
    $v = $_POST[$nom] ?? '';
    return is_string($v) && in_array($v, $permis, true) ? $v : $defaut;
}

/**
 * Plafond d'envois par adresse IP (hachée, jamais gardée en clair) sur une heure glissante par
 * tranche horaire. Lecture, contrôle et écriture sous un même verrou. Rend null si l'envoi passe,
 * sinon [code HTTP, message].
 */
function form_plafond(string $nom, int $plafond): ?array
{
    $cle = hash('sha256', ($_SERVER['REMOTE_ADDR'] ?? '') . '|e-ko.rip|' . $nom . '|' . gmdate('YmdH'));
    $prefixe = sys_get_temp_dir() . '/eko-rip-' . $nom . '-';
    foreach (glob($prefixe . '*') ?: [] as $vieux) {
        if (@filemtime($vieux) < time() - 3600) {
            @unlink($vieux);
        }
    }
    $f = @fopen($prefixe . $cle, 'c+');
    if ($f === false || !flock($f, LOCK_EX)) {
        return [503, 'Le service est momentanément indisponible. Écrivez-nous à ' . FORM_DESTINATAIRE . '.'];
    }
    $deja = (int) stream_get_contents($f);
    if ($deja >= $plafond) {
        flock($f, LOCK_UN);
        fclose($f);
        return [429, 'Trop d\'envois depuis cette connexion. Réessayez dans une heure, ou écrivez à ' . FORM_DESTINATAIRE . '.'];
    }
    ftruncate($f, 0);
    rewind($f);
    fwrite($f, (string) ($deja + 1));
    fflush($f);
    flock($f, LOCK_UN);
    fclose($f);
    return null;
}

/** Envoie le message à l'équipe ; Reply-To = l'adresse du visiteur, déjà validée. */
function form_envoyer(string $sujet, string $corps, string $repondre_a): bool
{
    $entetes = implode("\r\n", [
        'From: ' . mb_encode_mimeheader('Site E-KO RIP', 'UTF-8', 'Q') . ' <' . FORM_EXPEDITEUR . '>',
        'Reply-To: ' . $repondre_a,
        'MIME-Version: 1.0',
        'Content-Type: text/plain; charset=UTF-8',
        'Content-Transfer-Encoding: 8bit',
        'X-Mailer: e-ko.rip',
    ]);
    return mail(FORM_DESTINATAIRE, mb_encode_mimeheader($sujet, 'UTF-8', 'Q', "\r\n"), $corps, $entetes, '-f' . FORM_EXPEDITEUR);
}

/** Contrôles communs à tout envoi : méthode POST, origine e-ko.rip. */
function form_controles(bool $json, string $page, string $ancre_ok, string $ancre_erreur): void
{
    header('X-Content-Type-Options: nosniff');
    header('Cache-Control: no-store');
    if (($_SERVER['REQUEST_METHOD'] ?? '') !== 'POST') {
        header('Allow: POST');
        form_repondre(false, 'Méthode non autorisée.', 405, true, $page, $ancre_ok, $ancre_erreur);
    }
    $origine = $_SERVER['HTTP_ORIGIN'] ?? '';
    if ($origine !== '' && $origine !== 'https://e-ko.rip') {
        form_repondre(false, 'Origine refusée.', 403, $json, $page, $ancre_ok, $ancre_erreur);
    }
}
