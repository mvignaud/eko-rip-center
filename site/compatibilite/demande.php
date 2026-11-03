<?php
// E-KO RIP Center : réception du formulaire « Demande de pilote » (/compatibilite/).
// Seul script du site. Il envoie la demande par e-mail à l'équipe, ne garde rien sur le disque
// (hors un compteur par adresse IP hachée, contre les envois en rafale) et ne renvoie que du JSON,
// ou une redirection quand le navigateur poste le formulaire sans JavaScript.

declare(strict_types=1);

require dirname(__DIR__) . '/inc/formulaire.php';

const PAGE = '/compatibilite/';
const OK = 'demande-envoyee';
const KO = 'demande-erreur';
const DESTINATAIRE = FORM_DESTINATAIRE;

$json = str_contains($_SERVER['HTTP_ACCEPT'] ?? '', 'application/json');
form_controles($json, PAGE, OK, KO);

function repondre(bool $ok, string $message, int $code, bool $json): never
{
    form_repondre($ok, $message, $code, $json, PAGE, OK, KO);
}

function ligne(string $nom, int $max): string
{
    return form_ligne($nom, $max);
}

function texte(string $nom, int $max): string
{
    return form_texte($nom, $max);
}

// Pot de miel : un robot remplit le champ caché. On fait comme si tout allait bien.
if (ligne('website', 200) !== '') {
    repondre(true, 'Merci, votre demande est envoyée.', 200, $json);
}

$choix = fn (string $nom, array $permis, string $defaut) => form_choix($nom, $permis, $defaut);
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
$refus = form_plafond('demande', 5);
if ($refus !== null) {
    repondre(false, $refus[1], $refus[0], $json);
}

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

$envoye = form_envoyer('Demande de pilote : ' . $d['marque'] . ' ' . $d['modele'], $corps, $d['email']);
if (!$envoye) {
    error_log('e-ko.rip : échec de mail() pour une demande de pilote');
    repondre(false, 'L\'envoi a échoué. Écrivez-nous directement à ' . DESTINATAIRE . '.', 500, $json);
}

repondre(true, 'Merci, votre demande est envoyée. Nous vous répondons par e-mail.', 200, $json);
