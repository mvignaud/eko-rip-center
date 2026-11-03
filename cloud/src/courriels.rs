//! E-mails de la plate-forme : mise en page (un seul gabarit, HTML de messagerie à tables et
//! styles en ligne + version texte), réglages SMTP et envoi.
//!
//! Expéditeur : `no-reply@e-ko.fr` (nom affiché « E-KO RIP Cloud »). Les identifiants du
//! serveur SMTP sont saisis dans Administration (table `reglages`) ; les variables
//! d'environnement `EKO_CLOUD_SMTP_*` priment si présentes. Les envois partent hors de la
//! requête (`tokio::spawn`) et ne bloquent jamais une page ; en cas d'échec, une ligne de
//! journal sans secret. Sans SMTP réglé, ils sont ignorés (une ligne de journal).

use crate::base;
use crate::evenements::ResumeCompte;
use crate::portail::echapper;
use crate::relais::Relais;
use crate::temps::{date_fr, date_heure_fr, duree_fr};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const EXPEDITEUR_DEFAUT: &str = "no-reply@e-ko.fr";
pub const NOM_EXPEDITEUR: &str = "E-KO RIP Cloud";

/// Un message prêt à partir.
#[derive(Clone, Debug, PartialEq)]
pub struct Courriel {
    pub a: String,
    pub nom: String,
    pub sujet: String,
    pub texte: String,
    pub html: String,
}

// ————— Mise en page —————

/// Couleur d'accentuation du portail (`--accent`), claire puis sombre.
const ACCENT: &str = "#007aff";
const ACCENT_SOMBRE: &str = "#0a84ff";
const FONTE: &str = "-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif";

/// Un élément du corps d'un e-mail ; les textes sont bruts (échappés au rendu HTML).
pub enum Bloc {
    Paragraphe(String),
    Titre(String),
    /// Encadré gris : (libellé, valeur).
    Faits(Vec<(String, String)>),
    /// Chiffres en ligne : (valeur, libellé).
    Chiffres(Vec<(String, String)>),
    Liste(Vec<String>),
}

pub struct Contenu {
    pub sujet: String,
    /// Texte d'aperçu de la boîte de réception (caché dans le message).
    pub resume: String,
    pub titre: String,
    pub blocs: Vec<Bloc>,
    /// (libellé, adresse absolue) : l'unique bouton du message.
    pub bouton: Option<(String, String)>,
    /// Redonne l'adresse du bouton en clair (liens à usage unique).
    pub lien_en_clair: bool,
    /// Fin de la phrase « Vous recevez cet e-mail car … ».
    pub raison: String,
    /// Ajoute le lien vers les réglages des alertes et du résumé.
    pub preferences: bool,
}

fn une_ligne(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().trim().to_string()
}

fn html_bloc(b: &Bloc) -> String {
    match b {
        Bloc::Paragraphe(t) => format!(r#"<p class="texte" style="margin:0 0 16px;font:400 16px/1.55 {FONTE};color:#1d1d1f">{}</p>"#, echapper(t)),
        Bloc::Titre(t) => format!(r#"<h2 class="texte" style="margin:28px 0 12px;font:600 17px/1.3 {FONTE};color:#1d1d1f">{}</h2>"#, echapper(t)),
        Bloc::Faits(lignes) => {
            let l: String = lignes
                .iter()
                .map(|(k, v)| format!(r#"<tr><td class="doux" style="padding:5px 16px 5px 0;font:400 14px/1.4 {FONTE};color:#6e6e73;vertical-align:top;white-space:nowrap">{}</td><td class="texte" style="padding:5px 0;font:600 14px/1.4 {FONTE};color:#1d1d1f;vertical-align:top">{}</td></tr>"#, echapper(k), echapper(v)))
                .collect();
            format!(r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="margin:0 0 20px"><tr><td class="bloc" style="background:#f5f5f7;border-radius:12px;padding:12px 18px"><table role="presentation" cellpadding="0" cellspacing="0" border="0">{l}</table></td></tr></table>"#)
        }
        Bloc::Chiffres(cases) => {
            let largeur = 94 / cases.len().max(1);
            let c: String = cases
                .iter()
                .map(|(v, k)| format!(r#"<td class="bloc" width="{largeur}%" align="center" style="background:#f5f5f7;border-radius:12px;padding:14px 4px"><div class="texte" style="font:600 22px/1.2 {FONTE};color:#1d1d1f">{}</div><div class="doux" style="margin-top:4px;font:400 12px/1.3 {FONTE};color:#6e6e73">{}</div></td>"#, echapper(v), echapper(k)))
                .collect::<Vec<_>>()
                .join(r#"<td width="8" style="width:8px;font-size:0;line-height:0">&nbsp;</td>"#);
            format!(r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" style="margin:0 0 20px"><tr>{c}</tr></table>"#)
        }
        Bloc::Liste(items) => {
            let l: String = items.iter().map(|i| format!(r#"<li style="margin:0 0 6px">{}</li>"#, echapper(i))).collect();
            format!(r#"<ul class="texte" style="margin:0 0 16px;padding-left:20px;font:400 15px/1.5 {FONTE};color:#1d1d1f">{l}</ul>"#)
        }
    }
}

fn texte_bloc(b: &Bloc) -> String {
    match b {
        Bloc::Paragraphe(t) => format!("{t}\n\n"),
        Bloc::Titre(t) => format!("{t}\n{}\n", "-".repeat(t.chars().count().min(60))),
        Bloc::Faits(l) => format!("{}\n\n", l.iter().map(|(k, v)| format!("{k} : {v}")).collect::<Vec<_>>().join("\n")),
        Bloc::Chiffres(l) => format!("{}\n\n", l.iter().map(|(v, k)| format!("- {k} : {v}")).collect::<Vec<_>>().join("\n")),
        Bloc::Liste(l) => format!("{}\n\n", l.iter().map(|i| format!("- {i}")).collect::<Vec<_>>().join("\n")),
    }
}

/// Le gabarit commun à tous les e-mails.
pub fn rendre(url: &str, a: &str, nom: &str, c: &Contenu) -> Courriel {
    let corps: String = c.blocs.iter().map(html_bloc).collect();
    let bouton = c.bouton.as_ref().map_or(String::new(), |(libelle, lien)| {
        let secours = if c.lien_en_clair { format!(r#"<p class="doux" style="margin:0 0 16px;font:400 13px/1.5 {FONTE};color:#6e6e73;word-break:break-all">Si le bouton ne fonctionne pas, copiez cette adresse dans votre navigateur :<br><a class="lien" href="{0}" style="color:{ACCENT};text-decoration:underline">{0}</a></p>"#, echapper(lien)) } else { String::new() };
        format!(
            r#"<table role="presentation" cellpadding="0" cellspacing="0" border="0" style="margin:8px 0 24px"><tr><td align="center" bgcolor="{ACCENT}" class="bouton" style="background:{ACCENT};border-radius:10px"><a href="{}" target="_blank" style="display:inline-block;padding:13px 26px;font:600 16px/1.2 {FONTE};color:#ffffff;text-decoration:none;border-radius:10px">{}</a></td></tr></table>{secours}"#,
            echapper(lien),
            echapper(libelle)
        )
    });
    let prefs = if c.preferences { format!(r#" Pour choisir les alertes et résumés que vous recevez, ouvrez <a class="lien" href="{0}/compte" style="color:#6e6e73;text-decoration:underline">votre compte</a>."#, echapper(url)) } else { String::new() };
    let titre = echapper(&c.titre);
    let html = format!(
        r##"<!doctype html>
<html lang="fr" xmlns="http://www.w3.org/1999/xhtml">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<meta name="color-scheme" content="light dark">
<meta name="supported-color-schemes" content="light dark">
<title>{titre}</title>
<style>
:root {{ color-scheme: light dark; supported-color-schemes: light dark; }}
a {{ color: {ACCENT}; }}
@media (prefers-color-scheme: dark) {{
  .fond {{ background-color: #1c1c1e !important; }}
  .carte {{ background-color: #2c2c2e !important; border-color: #3a3a3c !important; }}
  .bloc {{ background-color: #3a3a3c !important; }}
  .texte, .marque {{ color: #f5f5f7 !important; }}
  .doux {{ color: #98989d !important; }}
  .bouton {{ background-color: {ACCENT_SOMBRE} !important; }}
  a.lien {{ color: {ACCENT_SOMBRE} !important; }}
}}
@media only screen and (max-width: 620px) {{
  .conteneur {{ width: 100% !important; }}
  .marge {{ padding: 28px 22px !important; }}
}}
</style>
</head>
<body class="fond" style="margin:0;padding:0;background-color:#f5f5f7;-webkit-text-size-adjust:100%">
<div style="display:none;max-height:0;overflow:hidden;opacity:0;color:transparent;mso-hide:all">{resume}{bourrage}</div>
<table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0" class="fond" style="background-color:#f5f5f7"><tr><td align="center" style="padding:32px 12px">
<table role="presentation" class="conteneur" width="600" cellpadding="0" cellspacing="0" border="0" style="width:600px;max-width:600px">
<tr><td style="padding:0 8px 18px"><table role="presentation" cellpadding="0" cellspacing="0" border="0"><tr><td style="padding-right:12px;vertical-align:middle"><img src="{url}/logo-courriel.png" width="40" height="40" alt="E-KO RIP" style="display:block;width:40px;height:40px;border:0;border-radius:9px"></td><td class="marque texte" style="vertical-align:middle;font:700 20px/1.2 {FONTE};color:#1d1d1f;letter-spacing:-.01em">E-KO RIP <span style="font-weight:400;color:{ACCENT}">Cloud</span></td></tr></table></td></tr>
<tr><td class="carte" style="background-color:#ffffff;border:1px solid #e5e5ea;border-radius:18px"><table role="presentation" width="100%" cellpadding="0" cellspacing="0" border="0"><tr><td class="marge" style="padding:36px 40px">
<h1 class="texte" style="margin:0 0 18px;font:600 26px/1.2 {FONTE};color:#1d1d1f;letter-spacing:-.015em">{titre}</h1>
{corps}{bouton}</td></tr></table></td></tr>
<tr><td class="doux" style="padding:22px 12px 0;font:400 12px/1.6 {FONTE};color:#86868b;text-align:center">Vous recevez cet e-mail car {raison}.{prefs}<br>E-KO RIP Cloud · <a class="lien" href="{url}" style="color:#86868b;text-decoration:underline">{url_texte}</a> · SAS GOLDEN PRINT</td></tr>
</table>
</td></tr></table>
</body>
</html>
"##,
        resume = echapper(&c.resume),
        // Sans ce bourrage, certaines messageries complètent l'aperçu avec le début du corps.
        bourrage = "&zwnj;&nbsp;".repeat(40),
        raison = echapper(&c.raison),
        url = echapper(url),
        url_texte = echapper(url.split("://").nth(1).unwrap_or(url)),
    );
    let mut texte = format!("E-KO RIP Cloud\n\n{}\n\n", c.titre);
    for b in &c.blocs {
        texte.push_str(&texte_bloc(b));
    }
    if let Some((libelle, lien)) = &c.bouton {
        texte.push_str(&format!("{libelle} :\n{lien}\n\n"));
    }
    texte.push_str(&format!("--\nVous recevez cet e-mail car {}.\n", c.raison));
    if c.preferences {
        texte.push_str(&format!("Alertes et résumés : {url}/compte\n"));
    }
    texte.push_str(&format!("E-KO RIP Cloud · {url} · SAS GOLDEN PRINT\n"));
    Courriel { a: a.into(), nom: une_ligne(nom), sujet: une_ligne(&c.sujet), texte, html }
}

fn pluriel(n: usize, un: &str, plusieurs: &str) -> String {
    format!("{n} {}", if n > 1 { plusieurs } else { un })
}

// ————— Les messages —————

/// 1. Mot de passe oublié.
pub fn mot_de_passe_oublie(url: &str, a: &str, nom: &str, lien: &str) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: "Réinitialiser votre mot de passe E-KO RIP Cloud".into(),
            resume: "Le lien est valable 1 heure.".into(),
            titre: "Nouveau mot de passe".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom},")),
                Bloc::Paragraphe("Vous avez demandé à réinitialiser le mot de passe de votre compte E-KO RIP Cloud. Le lien ci-dessous est valable 1 heure et ne peut servir qu'une fois.".into()),
            ],
            bouton: Some(("Choisir un nouveau mot de passe".into(), lien.into())),
            lien_en_clair: true,
            raison: "une réinitialisation de mot de passe a été demandée pour ce compte. Si ce n'est pas vous, ignorez ce message : votre mot de passe reste inchangé".into(),
            preferences: false,
        },
    )
}

/// 2. Confirmation de l'adresse à l'inscription.
pub fn confirmation_adresse(url: &str, a: &str, nom: &str, lien: &str) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: "Confirmez votre adresse e-mail E-KO RIP Cloud".into(),
            resume: "Un clic pour confirmer votre adresse et rattacher vos serveurs RIP.".into(),
            titre: "Confirmez votre adresse".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Bienvenue {nom},")),
                Bloc::Paragraphe("Pour rattacher le serveur RIP de votre atelier, confirmez que cette adresse est bien la vôtre. Le lien est valable 24 heures.".into()),
            ],
            bouton: Some(("Confirmer mon adresse".into(), lien.into())),
            lien_en_clair: true,
            raison: "un compte E-KO RIP Cloud vient d'être créé avec cette adresse. Si ce n'est pas vous, ignorez ce message".into(),
            preferences: false,
        },
    )
}

/// 3a. Site hors ligne depuis 15 minutes.
pub fn site_hors_ligne(url: &str, a: &str, nom: &str, site: &str, depuis: u64, maintenant: u64) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Le site « {site} » est hors ligne"),
            resume: format!("Plus de connexion depuis {}.", duree_fr(maintenant.saturating_sub(depuis))),
            titre: "Site hors ligne".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Le serveur RIP du site « {site} » ne s'est plus connecté à la plate-forme depuis plus de 15 minutes. L'accès à distance (postes hors de l'atelier, app mobile) est interrompu.")),
                Bloc::Faits(vec![("Site".into(), site.into()), ("Hors ligne depuis".into(), date_heure_fr(depuis)), ("Durée".into(), duree_fr(maintenant.saturating_sub(depuis)))]),
                Bloc::Paragraphe("Vérifiez que le serveur RIP est allumé et que la connexion Internet de l'atelier fonctionne. Vous serez prévenu dès son retour.".into()),
            ],
            bouton: Some(("Voir mes sites".into(), format!("{url}/sites"))),
            lien_en_clair: false,
            raison: format!("vous êtes propriétaire ou administrateur du site « {site} » et avez activé les alertes"),
            preferences: true,
        },
    )
}

/// 3b. Site de retour en ligne (après une alerte).
pub fn site_en_ligne(url: &str, a: &str, nom: &str, site: &str, debut: u64, fin: u64) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Le site « {site} » est de retour en ligne"),
            resume: format!("Coupure de {}.", duree_fr(fin.saturating_sub(debut))),
            titre: "Site de retour en ligne".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Le serveur RIP du site « {site} » est de nouveau connecté à la plate-forme : l'accès à distance fonctionne.")),
                Bloc::Faits(vec![("Site".into(), site.into()), ("Coupure".into(), format!("{} · {}", date_heure_fr(debut), duree_fr(fin.saturating_sub(debut)))), ("Retour".into(), date_heure_fr(fin))]),
            ],
            bouton: Some(("Voir mes sites".into(), format!("{url}/sites"))),
            lien_en_clair: false,
            raison: format!("vous avez reçu l'alerte « hors ligne » du site « {site} » et avez activé les alertes"),
            preferences: true,
        },
    )
}

/// 4. Nouvelle app connectée au compte.
pub fn nouvelle_app(url: &str, a: &str, nom: &str, appareil: &str, quand: u64) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: "Nouvelle app connectée à votre compte E-KO RIP Cloud".into(),
            resume: format!("{appareil} a été connecté à votre compte."),
            titre: "Nouvelle app connectée".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, une app vient d'être connectée à votre compte avec « Se connecter au cloud ». Elle peut ouvrir vos serveurs RIP avec vos droits.")),
                Bloc::Faits(vec![("Appareil".into(), appareil.into()), ("Date".into(), date_heure_fr(quand)), ("Compte".into(), a.into())]),
                Bloc::Paragraphe("Si c'est bien vous, il n'y a rien à faire. Sinon, déconnectez les apps depuis votre compte, puis changez votre mot de passe.".into()),
            ],
            bouton: Some(("Gérer mes apps connectées".into(), format!("{url}/compte"))),
            lien_en_clair: false,
            raison: "une app vient d'être connectée à votre compte (notification de sécurité)".into(),
            preferences: false,
        },
    )
}

/// 5. Résumé hebdomadaire d'un compte (semaine `[debut, fin[`).
pub fn resume_hebdomadaire(url: &str, r: &ResumeCompte, debut: u64, fin: u64) -> Courriel {
    let semaine = format!("du {} au {}", date_fr(debut), date_fr(fin - 86_400));
    let coupures: usize = r.sites.iter().map(|s| s.coupures).sum();
    let mut blocs = vec![Bloc::Paragraphe(format!("Bonjour {}, voici la semaine {semaine} pour {}.", r.nom, if r.sites.len() > 1 { format!("vos {} sites", r.sites.len()) } else { "votre site".to_string() }))];
    for s in &r.sites {
        blocs.push(Bloc::Titre(s.nom.clone()));
        let duree = |d: u64| if s.coupures == 0 { "—".to_string() } else { duree_fr(d) };
        blocs.push(Bloc::Chiffres(vec![(s.coupures.to_string(), if s.coupures > 1 { "coupures" } else { "coupure" }.into()), (duree(s.hors_ligne), "hors ligne".into()), (duree(s.plus_longue), "la plus longue".into()), (s.alertes.to_string(), if s.alertes > 1 { "alertes" } else { "alerte" }.into())]));
        if let Some((derniere, utilise, quota)) = s.sauvegardes {
            blocs.push(Bloc::Paragraphe(format!("Sauvegardes : {} · {} utilisés sur {}.", derniere.map_or("aucune sauvegarde réussie".to_string(), |d| format!("dernière réussie le {}", date_heure_fr(d))), crate::offres::taille(utilise), crate::offres::taille(quota))));
        }
    }
    blocs.push(Bloc::Titre("Apps connectées à votre compte cette semaine".into()));
    if r.applis.is_empty() {
        blocs.push(Bloc::Paragraphe("Aucune nouvelle app.".into()));
    } else {
        blocs.push(Bloc::Liste(r.applis.iter().map(|(appareil, quand)| format!("{appareil} · {}", date_heure_fr(*quand))).collect()));
    }
    rendre(
        url,
        &r.email,
        &r.nom,
        &Contenu {
            sujet: format!("Votre résumé E-KO RIP Cloud · semaine du {}", date_fr(debut)),
            resume: if coupures == 0 { "Aucune coupure cette semaine.".into() } else { format!("{} cette semaine.", pluriel(coupures, "coupure", "coupures")) },
            titre: "Votre semaine".into(),
            blocs,
            bouton: Some(("Ouvrir mes sites".into(), format!("{url}/sites"))),
            lien_en_clair: false,
            raison: "vous êtes propriétaire ou administrateur d'au moins un site et avez activé le résumé hebdomadaire".into(),
            preferences: true,
        },
    )
}

/// 6. E-mail d'essai (Administration).
pub fn essai(url: &str, a: &str, nom: &str, maintenant: u64) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: "E-mail d'essai E-KO RIP Cloud".into(),
            resume: "L'envoi d'e-mails fonctionne.".into(),
            titre: "L'envoi d'e-mails fonctionne".into(),
            blocs: vec![Bloc::Paragraphe(format!("Bonjour {nom}, ce message d'essai confirme que la plate-forme sait envoyer des e-mails (mot de passe oublié, confirmation d'adresse, alertes, résumé).")), Bloc::Faits(vec![("Envoyé le".into(), date_heure_fr(maintenant))])],
            bouton: Some(("Ouvrir E-KO RIP Cloud".into(), format!("{url}/sites"))),
            lien_en_clair: false,
            raison: "un administrateur de la plate-forme a demandé un e-mail d'essai".into(),
            preferences: false,
        },
    )
}

// ————— Abonnements, facturation et sauvegardes —————

/// Raison commune des e-mails de facturation.
const RAISON_FACTURATION: &str = "vous êtes titulaire d'un abonnement E-KO RIP Cloud (facturé par SAS GOLDEN PRINT)";

fn faits(l: &[(&str, String)]) -> Bloc {
    Bloc::Faits(l.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
}

/// 7. Abonnement souscrit, ou offre changée.
#[allow(clippy::too_many_arguments)]
pub fn abonnement_souscrit(url: &str, a: &str, nom: &str, offre: &str, prix_ht: &str, prochaine: Option<u64>, essai_fin: Option<u64>, change: bool, premiere_facture: Option<&str>) -> Courriel {
    let mut l = vec![("Offre", offre.to_string()), ("Prix", format!("{prix_ht} HT"))];
    if let Some(e) = essai_fin {
        l.push(("Essai gratuit jusqu'au", date_fr(e)));
    }
    if let Some(p) = prochaine {
        l.push(("Prochaine échéance", date_fr(p)));
    }
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: if change { format!("Votre offre E-KO RIP Cloud est maintenant « {offre} »") } else { format!("Bienvenue dans l'offre E-KO RIP Cloud « {offre} »") },
            resume: format!("{offre} · {prix_ht} HT"),
            titre: if change { "Offre modifiée".into() } else { "Abonnement confirmé".into() },
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, {}", if change { "votre changement d'offre est enregistré." } else { "merci : votre abonnement est enregistré." })),
                faits(&l),
                Bloc::Paragraphe(match premiere_facture {
                    Some(_) => "Votre première facture est émise : elle est disponible dans la page Abonnement du portail, comme les suivantes, émises au début de chaque période.".into(),
                    None => "Vos factures sont émises au début de chaque période et restent disponibles dans la page Abonnement du portail.".into(),
                }),
            ],
            bouton: Some(("Voir mon abonnement".into(), format!("{url}/abonnement"))),
            lien_en_clair: false,
            raison: RAISON_FACTURATION.into(),
            preferences: false,
        },
    )
}

/// 8. Facture émise.
pub fn facture_emise(url: &str, a: &str, nom: &str, reference: &str, montant_ttc: &str, lien_pdf: &str) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Votre facture E-KO RIP Cloud {reference}"),
            resume: format!("Montant : {montant_ttc} TTC."),
            titre: "Nouvelle facture".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, une nouvelle facture a été émise pour votre abonnement E-KO RIP Cloud.")),
                faits(&[("Facture", reference.to_string()), ("Montant", format!("{montant_ttc} TTC"))]),
                Bloc::Paragraphe("Elle est réglée automatiquement avec le moyen de paiement enregistré.".into()),
            ],
            bouton: Some(("Télécharger la facture (PDF)".into(), lien_pdf.into())),
            lien_en_clair: false,
            raison: RAISON_FACTURATION.into(),
            preferences: false,
        },
    )
}

/// Sujet, titre et texte d'une étape de paiement refusé (0 : refus ; 1 à 3 : relances de l'ERP).
pub fn etape_relance(etape: u8) -> (&'static str, &'static str, &'static str) {
    match etape {
        0 => ("Votre paiement E-KO RIP Cloud a été refusé", "Paiement refusé", "Le prélèvement de votre abonnement n'a pas pu être effectué. Une nouvelle tentative aura lieu automatiquement ; vous pouvez aussi mettre à jour votre moyen de paiement dès maintenant."),
        1 => ("Votre paiement E-KO RIP Cloud a échoué", "Paiement échoué", "Nous n'avons pas pu encaisser votre abonnement il y a quelques jours. Merci de vérifier votre moyen de paiement ou d'en enregistrer un nouveau : il suffit d'une minute."),
        2 => ("Action requise : votre abonnement E-KO RIP Cloud est en retard", "Action requise", "Le paiement de votre abonnement échoue toujours. Sans régularisation, votre compte reviendra à l'offre Gratuite et l'accès à distance des sites et machines au-delà de cette offre sera suspendu."),
        _ => ("Dernier rappel : votre abonnement E-KO RIP Cloud va être résilié", "Dernier rappel", "Dernier rappel avant résiliation : sans paiement, votre compte revient à l'offre Gratuite, l'accès à distance au-delà de cette offre est suspendu et l'historique des sauvegardes est réduit à la dernière version après 90 jours."),
    }
}

/// 9. Paiement refusé, puis relances J+3, J+7 et J+14 (formulations des étapes de relance de l'ERP).
pub fn paiement_refuse(url: &str, a: &str, nom: &str, etape: u8, montant_ttc: Option<&str>, raison_refus: Option<&str>) -> Courriel {
    let (sujet, titre, texte) = etape_relance(etape);
    let mut l = Vec::new();
    if let Some(m) = montant_ttc {
        l.push(("Montant dû", format!("{m} TTC")));
    }
    if let Some(r) = raison_refus.filter(|r| !r.is_empty()) {
        l.push(("Motif", r.to_string()));
    }
    if etape > 0 {
        l.push(("Relance", format!("{etape} sur 3")));
    }
    let mut blocs = vec![Bloc::Paragraphe(format!("Bonjour {nom},")), Bloc::Paragraphe(texte.into())];
    if !l.is_empty() {
        blocs.push(faits(&l));
    }
    rendre(
        url,
        a,
        nom,
        &Contenu { sujet: sujet.into(), resume: texte.chars().take(90).collect(), titre: titre.into(), blocs, bouton: Some(("Mettre à jour mon moyen de paiement".into(), format!("{url}/abonnement/paiement"))), lien_en_clair: false, raison: RAISON_FACTURATION.into(), preferences: false },
    )
}

/// 10. Retour à l'offre Gratuite (résiliation, ou impayé après la dernière relance).
pub fn retour_gratuit(url: &str, a: &str, nom: &str, impaye: bool) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: if impaye { "Abonnement E-KO RIP Cloud résilié pour impayé".into() } else { "Votre compte E-KO RIP Cloud est revenu à l'offre Gratuite".into() },
            resume: "Ce qui change pour vos sites et vos sauvegardes.".into(),
            titre: if impaye { "Abonnement résilié pour impayé".into() } else { "Retour à l'offre Gratuite".into() },
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, {}", if impaye { "faute de paiement après trois relances, votre abonnement est résilié et votre compte revient à l'offre Gratuite." } else { "votre abonnement est terminé : votre compte revient à l'offre Gratuite." })),
                Bloc::Liste(vec![
                    "L'offre Gratuite comprend 1 site, 1 imprimante et 1 découpeur.".into(),
                    "Au-delà, l'accès à distance des sites concernés est suspendu au bout de 14 jours (l'usage dans l'atelier n'est jamais coupé).".into(),
                    "Vos sauvegardes restent 90 jours, puis seule la dernière version est gardée.".into(),
                ]),
                Bloc::Paragraphe("Vous pouvez souscrire de nouveau à tout moment depuis la page Abonnement.".into()),
            ],
            bouton: Some(("Voir les offres".into(), format!("{url}/abonnement/offres"))),
            lien_en_clair: false,
            raison: RAISON_FACTURATION.into(),
            preferences: false,
        },
    )
}

/// 11. Fin de l'essai gratuit dans 3 jours.
pub fn fin_essai(url: &str, a: &str, nom: &str, offre: &str, fin: u64, moyen: &str) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Votre essai E-KO RIP Cloud se termine le {}", date_fr(fin)),
            resume: "Rien à faire si votre moyen de paiement est à jour.".into(),
            titre: "Fin de l'essai dans 3 jours".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, l'essai gratuit de l'offre « {offre} » se termine le {}. La première facture sera alors émise et réglée avec le moyen de paiement enregistré.", date_fr(fin))),
                faits(&[("Offre", offre.to_string()), ("Fin de l'essai", date_fr(fin)), ("Moyen de paiement", if moyen.is_empty() { "à enregistrer".to_string() } else { moyen.to_string() })]),
                Bloc::Paragraphe("Pour changer d'offre ou résilier avant la fin de l'essai, ouvrez la page Abonnement.".into()),
            ],
            bouton: Some(("Voir mon abonnement".into(), format!("{url}/abonnement"))),
            lien_en_clair: false,
            raison: RAISON_FACTURATION.into(),
            preferences: false,
        },
    )
}

/// 12. Quota de sauvegarde atteint à 80 % ou 100 %.
pub fn quota_sauvegarde(url: &str, a: &str, nom: &str, pourcent: u64, utilise: &str, quota: &str) -> Courriel {
    let plein = pourcent >= 100;
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: if plein { "Espace de sauvegarde E-KO RIP Cloud plein".into() } else { format!("Espace de sauvegarde E-KO RIP Cloud utilisé à {pourcent} %") },
            resume: format!("{utilise} utilisés sur {quota}."),
            titre: if plein { "Espace de sauvegarde plein".into() } else { "Espace de sauvegarde presque plein".into() },
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, {}", if plein { "votre espace de sauvegarde en ligne est plein : les nouvelles sauvegardes de vos serveurs RIP sont refusées jusqu'à ce que de la place se libère." } else { "votre espace de sauvegarde en ligne sera bientôt plein." })),
                Bloc::Chiffres(vec![(utilise.into(), "utilisés".into()), (quota.into(), "compris dans l'offre".into()), (format!("{pourcent} %"), "occupation".into())]),
                Bloc::Paragraphe("Les anciennes versions sont supprimées automatiquement selon la durée de conservation de votre offre. Pour plus d'espace, passez à une offre supérieure.".into()),
            ],
            bouton: Some(("Voir mon abonnement".into(), format!("{url}/abonnement"))),
            lien_en_clair: false,
            raison: "vous êtes propriétaire de sites sauvegardés sur E-KO RIP Cloud".into(),
            preferences: false,
        },
    )
}

/// 13. Sauvegarde nocturne absente deux nuits de suite.
pub fn sauvegarde_en_echec(url: &str, a: &str, nom: &str, site: &str, derniere: Option<u64>) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Sauvegarde du site « {site} » en échec"),
            resume: "Aucune sauvegarde reçue depuis deux nuits.".into(),
            titre: "Sauvegarde en échec".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Le serveur RIP du site « {site} » n'a envoyé aucune sauvegarde nocturne depuis deux nuits de suite.")),
                faits(&[("Site", site.to_string()), ("Dernière sauvegarde", derniere.map_or("aucune".to_string(), date_heure_fr))]),
                Bloc::Paragraphe("Vérifiez que le serveur RIP est allumé la nuit et connecté au cloud, et que l'espace de sauvegarde n'est pas plein.".into()),
            ],
            bouton: Some(("Voir les sauvegardes".into(), format!("{url}/sites"))),
            lien_en_clair: false,
            raison: format!("vous êtes propriétaire ou administrateur du site « {site} »"),
            preferences: false,
        },
    )
}

/// 14. Restauration effectuée.
pub fn restauration_effectuee(url: &str, a: &str, nom: &str, site: &str, qui: &str, quand: u64, version: u64) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Sauvegarde restaurée sur le site « {site} »"),
            resume: format!("Version du {} restaurée par {qui}.", date_heure_fr(version)),
            titre: "Restauration effectuée".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Une sauvegarde en ligne a été restaurée sur le serveur RIP du site « {site} ».")),
                faits(&[("Site", site.to_string()), ("Par", qui.to_string()), ("Le", date_heure_fr(quand)), ("Version restaurée", date_heure_fr(version))]),
                Bloc::Paragraphe("Si vous n'êtes pas à l'origine de cette restauration, prévenez le propriétaire du site.".into()),
            ],
            bouton: Some(("Voir mes sites".into(), format!("{url}/sites"))),
            lien_en_clair: false,
            raison: format!("vous êtes propriétaire ou administrateur du site « {site} »"),
            preferences: false,
        },
    )
}

/// 15. Offre dépassée : accès à distance suspendu dans 14 jours, ou suspendu.
pub fn offre_depassee(url: &str, a: &str, nom: &str, raison: &str, suspension: u64, suspendu: bool) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: if suspendu { "Accès à distance suspendu : offre E-KO RIP Cloud dépassée".into() } else { format!("Offre E-KO RIP Cloud dépassée : accès à distance suspendu le {}", date_fr(suspension)) },
            resume: raison.into(),
            titre: if suspendu { "Accès à distance suspendu".into() } else { "Offre dépassée".into() },
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, {}", if suspendu { "le délai de grâce est écoulé : l'accès à distance des sites concernés est suspendu. L'usage dans l'atelier n'est pas touché." } else { "votre parc dépasse ce que comprend votre offre. L'usage dans l'atelier n'est jamais touché, mais l'accès à distance des sites concernés sera suspendu à la fin du délai de grâce." })),
                faits(&[("Dépassement", raison.to_string()), (if suspendu { "Suspendu depuis le" } else { "Suspension le" }, date_fr(suspension))]),
                Bloc::Paragraphe("Pour rétablir l'accès, passez à une offre adaptée, ou retirez les machines ou sites en trop.".into()),
            ],
            bouton: Some(("Choisir une offre".into(), format!("{url}/abonnement/offres"))),
            lien_en_clair: false,
            raison: "vous êtes propriétaire de sites rattachés à E-KO RIP Cloud".into(),
            preferences: false,
        },
    )
}

/// 16. Mandat de prélèvement SEPA signé.
pub fn mandat_signe(url: &str, a: &str, nom: &str, rum: &str, iban_fin: &str, lien_pdf: &str) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: "Votre mandat de prélèvement SEPA E-KO RIP Cloud est signé".into(),
            resume: format!("Référence unique du mandat : {rum}."),
            titre: "Mandat SEPA signé".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, votre mandat de prélèvement SEPA au profit de SAS GOLDEN PRINT est signé électroniquement. Vos factures E-KO RIP Cloud seront prélevées sur ce compte.")),
                faits(&[("Référence (RUM)", rum.to_string()), ("Compte", format!("IBAN se terminant par {iban_fin}")), ("Créancier", "SAS GOLDEN PRINT".to_string())]),
                Bloc::Paragraphe("Vous pouvez contester un prélèvement auprès de votre banque dans les conditions de votre convention de compte.".into()),
            ],
            bouton: Some(("Télécharger le mandat (PDF)".into(), lien_pdf.into())),
            lien_en_clair: false,
            raison: RAISON_FACTURATION.into(),
            preferences: false,
        },
    )
}

// ————— Support —————

const RAISON_SUPPORT: &str = "vous avez ouvert une demande ou une discussion auprès du support E-KO RIP Cloud";

/// Texte coupé à `max` caractères (sur un mot si possible), avec « … » s'il a été coupé.
pub fn extrait(texte: &str, max: usize) -> String {
    let t: String = texte.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= max {
        return t;
    }
    let coupe: String = t.chars().take(max).collect();
    let coupe = coupe.rsplit_once(' ').map_or(coupe.as_str(), |(d, _)| d).trim_end_matches([',', ';', ':', '.', ' ']).to_string();
    format!("{coupe}…")
}

fn lien_demande(url: &str, code: &str) -> String {
    format!("{url}/support/{}", crate::portail::encoder(code))
}

/// 25. Demande reçue par le support.
pub fn support_demande_recue(url: &str, a: &str, nom: &str, code: &str, sujet: &str, file: &str, premiere_reponse: Option<u64>) -> Courriel {
    let mut l = vec![("Référence", code.to_string()), ("Sujet", une_ligne(sujet))];
    if !file.is_empty() {
        l.push(("File", file.to_string()));
    }
    if let Some(t) = premiere_reponse {
        l.push(("Première réponse attendue avant le", date_heure_fr(t)));
    }
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Votre demande au support E-KO RIP Cloud est reçue ({code})"),
            resume: format!("{code} · {}", extrait(sujet, 80)),
            titre: "Demande reçue".into(),
            blocs: vec![Bloc::Paragraphe(format!("Bonjour {nom}, l'équipe du support a bien reçu votre demande et vous répond dès que possible.")), faits(&l), Bloc::Paragraphe("Vous êtes prévenu par e-mail à chaque réponse. Vous pouvez aussi suivre la demande et répondre depuis le portail.".into())],
            bouton: Some(("Voir ma demande".into(), lien_demande(url, code))),
            lien_en_clair: false,
            raison: RAISON_SUPPORT.into(),
            preferences: false,
        },
    )
}

/// 26. Nouvelle réponse de l'équipe du support.
pub fn support_reponse(url: &str, a: &str, nom: &str, code: &str, sujet: &str, agent: &str, message: &str) -> Courriel {
    let qui = if agent.is_empty() { "L'équipe du support".to_string() } else { format!("{agent}, du support E-KO,") };
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Réponse du support E-KO RIP Cloud : {}", une_ligne(sujet)),
            resume: extrait(message, 110),
            titre: "Nouvelle réponse".into(),
            blocs: vec![Bloc::Paragraphe(format!("Bonjour {nom}, {qui} a répondu à votre demande {code}.")), faits(&[("Sujet", une_ligne(sujet)), ("Réponse", extrait(message, 600))]), Bloc::Paragraphe("Ouvrez la demande pour lire le message en entier et répondre.".into())],
            bouton: Some(("Lire et répondre".into(), lien_demande(url, code))),
            lien_en_clair: false,
            raison: RAISON_SUPPORT.into(),
            preferences: false,
        },
    )
}

/// 27. Demande résolue, avec le lien pour noter le support.
pub fn support_resolue(url: &str, a: &str, nom: &str, code: &str, sujet: &str) -> Courriel {
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Votre demande au support E-KO RIP Cloud est résolue ({code})"),
            resume: "Dites-nous si vous êtes satisfait : une note de 1 à 5.".into(),
            titre: "Demande résolue".into(),
            blocs: vec![
                Bloc::Paragraphe(format!("Bonjour {nom}, votre demande « {} » est marquée comme résolue.", une_ligne(sujet))),
                Bloc::Paragraphe("Votre réponse nous aide à nous améliorer : donnez une note de 1 à 5 au support, avec un commentaire si vous le souhaitez. Si le problème persiste, répondez simplement dans la demande : elle sera rouverte.".into()),
            ],
            bouton: Some(("Noter le support".into(), format!("{}?note=1#note", lien_demande(url, code)))),
            lien_en_clair: false,
            raison: RAISON_SUPPORT.into(),
            preferences: false,
        },
    )
}

/// 28. Discussion transformée en demande, avec la transcription `(auteur, texte)`.
pub fn support_discussion_convertie(url: &str, a: &str, nom: &str, code: &str, sujet: &str, transcription: &[(String, String)]) -> Courriel {
    let lignes: Vec<String> = transcription.iter().take(30).map(|(qui, texte)| format!("{qui} : {}", extrait(texte, 500))).collect();
    let mut blocs = vec![Bloc::Paragraphe(format!("Bonjour {nom}, personne n'a pu prendre votre discussion en direct à temps : elle est devenue la demande {code}. L'équipe du support vous répondra par e-mail et dans le portail."))];
    if !lignes.is_empty() {
        blocs.push(Bloc::Titre("Vos messages".into()));
        blocs.push(Bloc::Liste(lignes));
    }
    rendre(
        url,
        a,
        nom,
        &Contenu {
            sujet: format!("Votre discussion est devenue la demande {code}"),
            resume: format!("{code} · {}", extrait(sujet, 80)),
            titre: "Discussion transformée en demande".into(),
            blocs,
            bouton: Some(("Voir ma demande".into(), lien_demande(url, code))),
            lien_en_clair: false,
            raison: RAISON_SUPPORT.into(),
            preferences: false,
        },
    )
}

/// Tous les messages avec des données d'exemple (`eko-rip-cloud apercu-courriels <dossier>`).
pub fn apercus(url: &str) -> Vec<(&'static str, Courriel)> {
    use crate::evenements::ResumeSite;
    // Lundi 2 novembre 2026, 8 h 00 (Paris).
    let lundi = crate::temps::jours_depuis_civil(2026, 11, 2) as u64 * 86_400 + 7 * 3600;
    let r = ResumeCompte {
        nom: "Atelier Dupont".into(),
        email: "contact@atelier-dupont.fr".into(),
        sites: vec![
            ResumeSite { nom: "Imprimerie Dupont · Lyon".into(), coupures: 2, hors_ligne: 3 * 3600 + 40 * 60, plus_longue: 3 * 3600, alertes: 1, sauvegardes: Some((Some(lundi - 6 * 3600), 3 * crate::offres::GO / 2, 25 * crate::offres::GO)) },
            ResumeSite { nom: "Atelier de Nantes".into(), coupures: 0, hors_ligne: 0, plus_longue: 0, alertes: 0, sauvegardes: None },
        ],
        applis: vec![("MacBook Pro de Claire".into(), lundi - 4 * 86_400 + 3600), ("iPhone de Claire".into(), lundi - 86_400)],
    };
    vec![
        ("mot-de-passe-oublie", mot_de_passe_oublie(url, "contact@atelier-dupont.fr", "Atelier Dupont", &format!("{url}/mot-de-passe/nouveau?jeton=Zk3pQ9vT2mXw8LrB5nYd1cHsJf7aUeGo4iVtKxNb0Cg"))),
        ("confirmation-adresse", confirmation_adresse(url, "contact@atelier-dupont.fr", "Atelier Dupont", &format!("{url}/email/confirmer?jeton=Zk3pQ9vT2mXw8LrB5nYd1cHsJf7aUeGo4iVtKxNb0Cg"))),
        ("site-hors-ligne", site_hors_ligne(url, "contact@atelier-dupont.fr", "Atelier Dupont", "Imprimerie Dupont · Lyon", lundi - 3 * 3600 - 900, lundi - 3 * 3600)),
        ("site-en-ligne", site_en_ligne(url, "contact@atelier-dupont.fr", "Atelier Dupont", "Imprimerie Dupont · Lyon", lundi - 3 * 3600 - 900, lundi - 2 * 3600)),
        ("nouvelle-app", nouvelle_app(url, "contact@atelier-dupont.fr", "Atelier Dupont", "MacBook Pro de Claire", lundi - 600)),
        ("resume-hebdomadaire", resume_hebdomadaire(url, &r, lundi - 7 * 86_400, lundi)),
        ("essai", essai(url, "contact@atelier-dupont.fr", "Atelier Dupont", lundi)),
        ("abonnement-souscrit", abonnement_souscrit(url, "contact@atelier-dupont.fr", "Atelier Dupont", "Atelier", "19,00 € par mois", Some(lundi + 30 * 86_400), Some(lundi + 30 * 86_400), false, None)),
        ("abonnement-change", abonnement_souscrit(url, "contact@atelier-dupont.fr", "Atelier Dupont", "Pro", "490,00 € par an", Some(lundi + 365 * 86_400), None, true, Some("FA-2026-0042"))),
        ("facture-emise", facture_emise(url, "contact@atelier-dupont.fr", "Atelier Dupont", "FA-2026-00142", "22,80 €", &format!("{url}/abonnement/factures/FA-2026-00142.pdf"))),
        ("paiement-refuse", paiement_refuse(url, "contact@atelier-dupont.fr", "Atelier Dupont", 0, Some("22,80 €"), Some("Provision insuffisante"))),
        ("relance-1", paiement_refuse(url, "contact@atelier-dupont.fr", "Atelier Dupont", 1, Some("22,80 €"), None)),
        ("relance-2", paiement_refuse(url, "contact@atelier-dupont.fr", "Atelier Dupont", 2, Some("22,80 €"), None)),
        ("relance-3", paiement_refuse(url, "contact@atelier-dupont.fr", "Atelier Dupont", 3, Some("22,80 €"), None)),
        ("resiliation-impaye", retour_gratuit(url, "contact@atelier-dupont.fr", "Atelier Dupont", true)),
        ("retour-gratuit", retour_gratuit(url, "contact@atelier-dupont.fr", "Atelier Dupont", false)),
        ("fin-essai", fin_essai(url, "contact@atelier-dupont.fr", "Atelier Dupont", "Atelier", lundi + 3 * 86_400, "Carte se terminant par 4242")),
        ("quota-80", quota_sauvegarde(url, "contact@atelier-dupont.fr", "Atelier Dupont", 80, "20 Go", "25 Go")),
        ("quota-100", quota_sauvegarde(url, "contact@atelier-dupont.fr", "Atelier Dupont", 100, "25 Go", "25 Go")),
        ("sauvegarde-echec", sauvegarde_en_echec(url, "contact@atelier-dupont.fr", "Atelier Dupont", "Imprimerie Dupont · Lyon", Some(lundi - 3 * 86_400))),
        ("restauration", restauration_effectuee(url, "contact@atelier-dupont.fr", "Atelier Dupont", "Imprimerie Dupont · Lyon", "Claire Dupont", lundi, lundi - 86_400)),
        ("offre-depassee", offre_depassee(url, "contact@atelier-dupont.fr", "Atelier Dupont", "3 imprimante(s) et 1 découpeur(s) pour 1 imprimante et 1 découpeur compris dans l'offre Gratuit", lundi + 14 * 86_400, false)),
        ("acces-suspendu", offre_depassee(url, "contact@atelier-dupont.fr", "Atelier Dupont", "2 sites rattachés pour 1 compris dans l'offre Gratuit", lundi, true)),
        ("mandat-signe", mandat_signe(url, "contact@atelier-dupont.fr", "Atelier Dupont", "GP-SEPA-000123", "0189", &format!("{url}/abonnement/mandat.pdf"))),
        ("support-demande-recue", support_demande_recue(url, "contact@atelier-dupont.fr", "Atelier Dupont", "TKT-202611-4H8K2P", "Le RIP ne voit plus l'imprimante", "Technique", Some(lundi + 9 * 3600))),
        ("support-reponse", support_reponse(url, "contact@atelier-dupont.fr", "Atelier Dupont", "TKT-202611-4H8K2P", "Le RIP ne voit plus l'imprimante", "Camille", "Bonjour, merci pour le journal. Réinstallez le pilote depuis la page Machines, puis relancez la détection. Dites-moi si l'imprimante réapparaît.")),
        ("support-resolue", support_resolue(url, "contact@atelier-dupont.fr", "Atelier Dupont", "TKT-202611-4H8K2P", "Le RIP ne voit plus l'imprimante")),
        ("support-discussion-convertie", support_discussion_convertie(url, "contact@atelier-dupont.fr", "Atelier Dupont", "TKT-202611-4H9M3Q", "Question sur la facture", &[("Vous".to_string(), "Bonjour, j'ai une question sur ma dernière facture.".to_string()), ("Support E-KO".to_string(), "Le support est fermé pour le moment : votre message devient une demande.".to_string())])),
    ]
}

// ————— Réglages SMTP —————

/// Serveur SMTP : réglages saisis dans Administration, ou variables `EKO_CLOUD_SMTP_*`.
#[derive(Clone, PartialEq)]
pub struct Smtp {
    pub hote: String,
    pub port: u16,
    /// TLS direct (port 465) ; sinon STARTTLS (port 587). Jamais en clair.
    pub tls_direct: bool,
    pub identifiant: String,
    pub mot_de_passe: String,
    pub expediteur: String,
}

impl std::fmt::Debug for Smtp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Smtp").field("hote", &self.hote).field("port", &self.port).field("tls_direct", &self.tls_direct).field("identifiant", &self.identifiant).field("mot_de_passe", &"…").field("expediteur", &self.expediteur).finish()
    }
}

/// Réglages de `reglages` (`smtp_hote`, `smtp_port`, `smtp_securite` = `starttls` ou `tls`,
/// `smtp_identifiant`, `smtp_mot_de_passe`, `smtp_expediteur`) ; `env` donne les variables
/// d'environnement, qui priment (`EKO_CLOUD_SMTP_HOTE`, `_PORT`, `_SECURITE`, `_IDENTIFIANT`,
/// `_MOT_DE_PASSE`, `_EXPEDITEUR`). `None` tant que l'hôte manque.
pub fn lire_smtp(c: &Connection, env: &dyn Fn(&str) -> Option<String>) -> Option<Smtp> {
    let valeur = |cle: &str, var: &str| env(var).filter(|v| !v.trim().is_empty()).or_else(|| base::reglage(c, cle).ok().flatten()).map(|v| v.trim().to_string());
    let hote = valeur("smtp_hote", "EKO_CLOUD_SMTP_HOTE")?;
    let tls_direct = matches!(valeur("smtp_securite", "EKO_CLOUD_SMTP_SECURITE").as_deref(), Some("tls"));
    let port = valeur("smtp_port", "EKO_CLOUD_SMTP_PORT").and_then(|p| p.parse::<u16>().ok()).filter(|p| *p > 0).unwrap_or(if tls_direct { 465 } else { 587 });
    let expediteur = valeur("smtp_expediteur", "EKO_CLOUD_SMTP_EXPEDITEUR").filter(|e| base::email_valide(e)).unwrap_or_else(|| EXPEDITEUR_DEFAUT.into());
    Some(Smtp { hote, port, tls_direct, identifiant: valeur("smtp_identifiant", "EKO_CLOUD_SMTP_IDENTIFIANT").unwrap_or_default(), mot_de_passe: valeur("smtp_mot_de_passe", "EKO_CLOUD_SMTP_MOT_DE_PASSE").unwrap_or_default(), expediteur })
}

fn env_processus(nom: &str) -> Option<String> {
    std::env::var(nom).ok()
}

/// Message `lettre` : texte + HTML, expéditeur « E-KO RIP Cloud <no-reply@e-ko.fr> ».
fn construire(expediteur: &str, c: &Courriel) -> Result<lettre::Message, String> {
    use lettre::message::{Mailbox, MultiPart};
    let de = Mailbox::new(Some(NOM_EXPEDITEUR.into()), expediteur.parse().map_err(|_| "expéditeur invalide".to_string())?);
    let a = Mailbox::new((!c.nom.is_empty()).then(|| c.nom.clone()), c.a.parse().map_err(|_| "destinataire invalide".to_string())?);
    lettre::Message::builder().from(de).to(a).subject(c.sujet.clone()).multipart(MultiPart::alternative_plain_html(c.texte.clone(), c.html.clone())).map_err(|e| format!("message invalide : {e}"))
}

async fn envoyer_smtp(s: &Smtp, c: &Courriel) -> Result<(), String> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};
    let message = construire(&s.expediteur, c)?;
    let b = if s.tls_direct { AsyncSmtpTransport::<Tokio1Executor>::relay(&s.hote) } else { AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&s.hote) }.map_err(|e| format!("serveur SMTP {} : {e}", s.hote))?;
    let mut b = b.port(s.port).timeout(Some(Duration::from_secs(20)));
    if !s.identifiant.is_empty() {
        b = b.credentials(Credentials::new(s.identifiant.clone(), s.mot_de_passe.clone()));
    }
    b.build().send(message).await.map(|_| ()).map_err(|e| format!("serveur SMTP {}:{} : {e}", s.hote, s.port))
}

// ————— Envoi —————

/// Où partent les messages : le serveur SMTP, ou la mémoire (tests).
pub enum Expediteur {
    Smtp,
    Memoire(Arc<Mutex<Vec<Courriel>>>),
}

impl Relais {
    /// Pour les tests : garde les messages en mémoire au lieu de les envoyer. Rend la boîte.
    pub fn utiliser_memoire(&self) -> Arc<Mutex<Vec<Courriel>>> {
        let boite = Arc::new(Mutex::new(Vec::new()));
        *self.courriels.lock().unwrap_or_else(|p| p.into_inner()) = Expediteur::Memoire(boite.clone());
        boite
    }

    fn smtp(&self) -> Option<Smtp> {
        lire_smtp(&self.base(), &env_processus)
    }

    /// Les e-mails partent-ils (SMTP réglé, ou transport mémoire) ?
    pub fn courriels_actifs(&self) -> bool {
        matches!(*self.courriels.lock().unwrap_or_else(|p| p.into_inner()), Expediteur::Memoire(_)) || self.smtp().is_some()
    }

    /// Envoie hors de la requête. Sans SMTP, le message est ignoré (une ligne de journal).
    pub fn envoyer(&self, c: Courriel) {
        let smtp = {
            let e = self.courriels.lock().unwrap_or_else(|p| p.into_inner());
            if let Expediteur::Memoire(boite) = &*e {
                boite.lock().unwrap_or_else(|p| p.into_inner()).push(c);
                return;
            }
            self.smtp()
        };
        let Some(smtp) = smtp else {
            eprintln!("e-mail ignoré (SMTP non réglé) : {}", c.sujet);
            return;
        };
        let Ok(rt) = tokio::runtime::Handle::try_current() else { return };
        rt.spawn(async move {
            if let Err(e) = envoyer_smtp(&smtp, &c).await {
                eprintln!("e-mail « {} » non envoyé : {e}", c.sujet);
            }
        });
    }

    /// Comme `envoyer`, mais attend le résultat (e-mail d'essai de l'administration).
    pub async fn envoyer_et_attendre(&self, c: Courriel) -> Result<(), String> {
        let smtp = {
            let e = self.courriels.lock().unwrap_or_else(|p| p.into_inner());
            if let Expediteur::Memoire(boite) = &*e {
                boite.lock().unwrap_or_else(|p| p.into_inner()).push(c);
                return Ok(());
            }
            self.smtp()
        };
        let Some(smtp) = smtp else { return Err("SMTP non réglé : renseigner l'hôte du serveur d'envoi.".into()) };
        envoyer_smtp(&smtp, &c).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gabarit_commun() {
        let c = site_hors_ligne("https://platform.e-ko.rip", "a@b.fr", "Élodie <Dupont>", "Atelier <script>alert(1)</script>", 1_000_000, 1_000_900);
        assert!(c.html.contains("max-width:600px") && c.html.contains("prefers-color-scheme: dark") && c.html.contains(r#"name="color-scheme""#));
        assert!(c.html.contains("E-KO RIP") && c.html.matches("<img").count() == 1 && c.html.contains(r#"src="https://platform.e-ko.rip/logo-courriel.png""#), "logo servi par la plate-forme, marque aussi en texte");
        assert!(c.html.contains("&lt;script&gt;") && !c.html.contains("<script>"), "contenu utilisateur échappé");
        assert_eq!(c.html.matches(r#"class="bouton""#).count(), 1, "un seul bouton");
        assert!(c.html.contains(r#"href="https://platform.e-ko.rip/sites""#) && c.html.contains("https://platform.e-ko.rip/compte"));
        assert!(c.html.contains("Vous recevez cet e-mail car") && c.texte.contains("Vous recevez cet e-mail car"));
        assert!(c.texte.contains("https://platform.e-ko.rip/compte") && c.texte.contains("Atelier <script>alert(1)</script>"), "version texte non échappée");
        assert!(c.sujet.starts_with("Le site « Atelier"));
    }

    #[test]
    fn sujet_sur_une_ligne() {
        let c = site_hors_ligne("https://x.fr", "a@b.fr", "N", "ligne1\r\nBcc: pirate@x.fr", 0, 1000);
        assert!(!c.sujet.contains('\n') && !c.sujet.contains('\r'));
    }

    #[test]
    fn lien_de_reinitialisation_en_clair() {
        let c = mot_de_passe_oublie("https://x.fr", "a@b.fr", "A", "https://x.fr/mot-de-passe/nouveau?jeton=abc");
        assert!(c.texte.contains("https://x.fr/mot-de-passe/nouveau?jeton=abc") && c.html.contains("copiez cette adresse"));
        assert!(c.html.contains("1 heure"));
    }

    #[test]
    fn apercus_complets() {
        let v = apercus("https://platform.e-ko.rip");
        assert_eq!(v.len(), 28);
        let noms: Vec<&str> = v.iter().map(|(n, _)| *n).collect();
        for n in ["abonnement-souscrit", "facture-emise", "paiement-refuse", "relance-1", "relance-2", "relance-3", "resiliation-impaye", "fin-essai", "quota-80", "quota-100", "sauvegarde-echec", "restauration", "offre-depassee", "acces-suspendu", "mandat-signe", "support-demande-recue", "support-reponse", "support-resolue", "support-discussion-convertie"] {
            assert!(noms.contains(&n), "aperçu {n} absent");
        }
        for (_, c) in &v {
            assert!(c.html.starts_with("<!doctype html>") && !c.texte.is_empty() && c.html.contains("display:none"), "{}", c.sujet);
        }
    }

    fn base_test() -> (std::path::PathBuf, Connection) {
        let d = std::env::temp_dir().join(format!("eko-cloud-smtp-{}", crate::aleatoire_hex(6)));
        let c = base::ouvrir(&d).unwrap();
        (d, c)
    }

    #[test]
    fn reglages_smtp_et_variables() {
        let (d, c) = base_test();
        let aucune = |_: &str| None;
        assert_eq!(lire_smtp(&c, &aucune), None, "sans hôte : pas d'envoi");
        base::regler(&c, "smtp_hote", "smtp.exemple.fr").unwrap();
        let s = lire_smtp(&c, &aucune).unwrap();
        assert_eq!((s.port, s.tls_direct, s.expediteur.as_str()), (587, false, "no-reply@e-ko.fr"));
        base::regler(&c, "smtp_securite", "tls").unwrap();
        base::regler(&c, "smtp_mot_de_passe", "secret-de-test").unwrap();
        let s = lire_smtp(&c, &aucune).unwrap();
        assert_eq!((s.port, s.tls_direct), (465, true));
        assert!(!format!("{s:?}").contains("secret-de-test"), "le mot de passe n'apparaît pas dans les journaux");
        // Les variables d'environnement priment.
        let env = |n: &str| match n {
            "EKO_CLOUD_SMTP_HOTE" => Some("autre.exemple.fr".to_string()),
            "EKO_CLOUD_SMTP_PORT" => Some("2525".to_string()),
            "EKO_CLOUD_SMTP_EXPEDITEUR" => Some("pas un e-mail".to_string()),
            _ => None,
        };
        let s = lire_smtp(&c, &env).unwrap();
        assert_eq!((s.hote.as_str(), s.port, s.expediteur.as_str()), ("autre.exemple.fr", 2525, "no-reply@e-ko.fr"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn message_lettre() {
        let c = nouvelle_app("https://x.fr", "a@b.fr", "Élodie", "Mac", 0);
        let m = construire("no-reply@e-ko.fr", &c).unwrap();
        let brut = String::from_utf8_lossy(&m.formatted()).to_string();
        assert!(brut.contains("no-reply@e-ko.fr") && brut.contains("multipart/alternative") && brut.contains("text/html"), "{brut}");
        assert!(construire("pas une adresse", &c).is_err());
    }
}
