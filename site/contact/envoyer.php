<?php
// E-KO RIP Center : réception du formulaire de contact (/contact/).
// Envoie le message par e-mail à l'équipe (Reply-To = le visiteur) et ne garde rien.
// Réponse JSON avec JavaScript ; sans, renvoi sur /contact/#message-envoye ou #message-erreur.

declare(strict_types=1);

require dirname(__DIR__) . '/inc/formulaire.php';

const PAGE = '/contact/';
const OK = 'message-envoye';
const KO = 'message-erreur';

$json = str_contains($_SERVER['HTTP_ACCEPT'] ?? '', 'application/json');
form_controles($json, PAGE, OK, KO);

// Pot de miel : un robot remplit le champ caché. On fait comme si tout allait bien.
if (form_ligne('website', 200) !== '') {
    form_repondre(true, 'Merci, votre message est envoyé.', 200, $json, PAGE, OK, KO);
}

$sujets = [
    'essai' => 'Essai et installation',
    'demo' => 'Demande de démonstration',
    'technique' => 'Question technique',
    'cloud' => 'Cloud et tarifs',
    'machine' => 'Machine ou pilote',
    'partenariat' => 'Partenariat ou revendeur',
    'presse' => 'Presse',
    'autre' => 'Autre',
];

$d = [
    'sujet' => form_choix('sujet', array_keys($sujets), 'autre'),
    'nom' => form_ligne('nom', 100),
    'societe' => form_ligne('societe', 120),
    'email' => form_ligne('email', 160),
    'telephone' => form_ligne('telephone', 30),
    'machines' => form_ligne('machines', 200),
    'message' => form_texte('message', 5000),
    'accord' => ($_POST['accord'] ?? '') === 'oui',
];

if ($d['nom'] === '' || $d['message'] === '') {
    form_repondre(false, 'Indiquez au moins votre nom et votre message.', 422, $json, PAGE, OK, KO);
}
if (filter_var($d['email'], FILTER_VALIDATE_EMAIL) === false) {
    form_repondre(false, 'L\'adresse e-mail ne semble pas valide.', 422, $json, PAGE, OK, KO);
}
if (!$d['accord']) {
    form_repondre(false, 'Cochez la case qui nous autorise à vous répondre.', 422, $json, PAGE, OK, KO);
}

$refus = form_plafond('contact', 5);
if ($refus !== null) {
    form_repondre(false, $refus[1], $refus[0], $json, PAGE, OK, KO);
}

$corps = implode("\n", [
    'Nouveau message depuis https://e-ko.rip/contact/',
    '',
    'Sujet : ' . $sujets[$d['sujet']],
    'Nom : ' . $d['nom'],
    'Société : ' . ($d['societe'] ?: '-'),
    'E-mail : ' . $d['email'],
    'Téléphone : ' . ($d['telephone'] ?: '-'),
    'Machines : ' . ($d['machines'] ?: '-'),
    '',
    'MESSAGE',
    $d['message'],
    '',
    '-- ',
    'Envoyé le ' . gmdate('Y-m-d H:i') . ' UTC. Répondre à ce message écrit directement au visiteur.',
]);

if (!form_envoyer('Contact e-ko.rip : ' . $sujets[$d['sujet']] . ' (' . $d['nom'] . ')', $corps, $d['email'])) {
    error_log('e-ko.rip : échec de mail() pour un message de contact');
    form_repondre(false, 'L\'envoi a échoué. Écrivez-nous directement à ' . FORM_DESTINATAIRE . '.', 500, $json, PAGE, OK, KO);
}

form_repondre(true, 'Merci, votre message est envoyé. Nous vous répondons par e-mail, en général sous un jour ouvré.', 200, $json, PAGE, OK, KO);
