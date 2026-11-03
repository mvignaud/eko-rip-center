"""Menu et pied de page communs à toutes les pages de https://e-ko.rip.

Une seule source pour le menu : outils/generer-site.py le pose dans les pages écrites à la main
(entre <!-- entete --> et <!-- /entete -->, <!-- pied --> et <!-- /pied -->) et dans le blog ;
outils/generer-documentation.py l'utilise pour la documentation (FR et EN).
"""
import html

GITHUB = "https://github.com/mvignaud/eko-rip-center"

# (titre, [(adresse, libellé, description courte)]) ; une rubrique à une seule entrée est un lien simple.
MENU = {
    "fr": [
        ("Logiciel", [
            ("/#fonctionnalites", "Fonctionnalités", "Rendu, couleur, blanc, découpe, file"),
            ("/#fonctionnement", "Fonctionnement", "Le trajet d'un fichier, étape par étape"),
            ("/#captures", "Captures", "Les clients Mac, Linux et Windows"),
            ("/app/", "Application mobile", "La file et les alertes dans la poche"),
            ("/#open-source", "Open source", "Code ouvert sous licence AGPL-3.0"),
            ("/feuille-de-route/", "Feuille de route", "Fait, en cours, planifié"),
            ("/changelog/", "Nouveautés", "Le journal des versions"),
        ]),
        ("Machines", [
            ("/compatibilite/", "Machines compatibles", "Imprimantes et découpeuses pilotées"),
            ("/compatibilite/#demande", "Demander un pilote", "Votre machine n'est pas dans la liste"),
            ("/prerequis/", "Pré-requis", "Serveur, postes et réseau"),
        ]),
        ("Cloud", [
            ("/cloud/", "Cloud et tarifs", "Piloter l'atelier de partout, sans VPN"),
        ]),
        ("Ressources", [
            ("/documentation/", "Documentation", "Installer, régler, dépanner"),
            ("/blog/", "Blog", "Guides d'atelier et comparatifs"),
            ("/blog/categorie/comparatifs/", "Comparatifs", "E-KO RIP face aux autres RIP"),
            (GITHUB, "GitHub", "Le code source et les tickets"),
            ("/contact/", "Contact", "Une question, une démonstration"),
        ]),
    ],
    "en": [
        ("Software", [
            ("/#fonctionnalites", "Features", "Rendering, colour, white, cutting, queue"),
            ("/#fonctionnement", "How it works", "A file's journey, step by step"),
            ("/#captures", "Screenshots", "Mac, Linux and Windows clients"),
            ("/app/", "Mobile app", "Queue and alerts in your pocket"),
            ("/#open-source", "Open source", "Open code under the AGPL-3.0"),
            ("/feuille-de-route/", "Roadmap", "Done, in progress, planned"),
            ("/changelog/", "What's new", "Release notes"),
        ]),
        ("Machines", [
            ("/compatibilite/", "Supported machines", "Printers and cutters driven"),
            ("/compatibilite/#demande", "Request a driver", "Your machine is not listed"),
            ("/prerequis/", "Requirements", "Server, workstations and network"),
        ]),
        ("Cloud", [
            ("/cloud/", "Cloud and pricing", "Run the shop from anywhere, no VPN"),
        ]),
        ("Resources", [
            ("/documentation/en/", "Documentation", "Install, set up, troubleshoot"),
            ("/blog/", "Blog (French)", "Shop guides and comparisons"),
            (GITHUB, "GitHub", "Source code and issues"),
            ("/contact/", "Contact", "A question, a demo"),
        ]),
    ],
}

TEXTES = {
    "fr": {"menu": "Menu", "telecharger": "Télécharger", "contact": "Contact", "accueil": "E-KO RIP Center, accueil",
           "slogan": "Le RIP grand format libre qui pilote tout l'atelier : rendu, couleur, blanc, découpe et machines. "
                     "Conçu et éprouvé dans un atelier d'impression du Gers.",
           "colonnes": [
               ("Logiciel", [("/#fonctionnalites", "Fonctionnalités"), ("/telecharger/", "Télécharger"), ("/app/", "Application mobile"),
                             ("/cloud/", "Cloud et tarifs"), ("/feuille-de-route/", "Feuille de route"), ("/changelog/", "Nouveautés")]),
               ("Ressources", [("/documentation/", "Documentation"), ("/blog/", "Blog"), ("/blog/categorie/comparatifs/", "Comparatifs"),
                               ("/compatibilite/", "Machines compatibles"), ("/prerequis/", "Pré-requis"), ("/blog/rss.xml", "Flux RSS du blog")]),
               ("E-KO RIP", [("/contact/", "Contact"), ("/compatibilite/#demande", "Demander un pilote"), (GITHUB, "GitHub"),
                             ("/mentions-legales/", "Mentions légales"), ("/app/confidentialite/", "Confidentialité de l'application")]),
           ],
           "droits": "Logiciel libre sous licence AGPL-3.0"},
    "en": {"menu": "Menu", "telecharger": "Download", "contact": "Contact", "accueil": "E-KO RIP Center, home",
           "slogan": "The open-source wide-format RIP that runs the whole print shop: rendering, colour, white ink, cutting and machines. "
                     "Designed and proven in a print shop in south-west France.",
           "colonnes": [
               ("Software", [("/#fonctionnalites", "Features"), ("/telecharger/", "Download"), ("/app/", "Mobile app"),
                             ("/cloud/", "Cloud and pricing"), ("/feuille-de-route/", "Roadmap"), ("/changelog/", "What's new")]),
               ("Resources", [("/documentation/en/", "Documentation"), ("/blog/", "Blog (French)"), ("/compatibilite/", "Supported machines"),
                              ("/prerequis/", "Requirements")]),
               ("E-KO RIP", [("/contact/", "Contact"), (GITHUB, "GitHub"), ("/mentions-legales/", "Legal notice")]),
           ],
           "droits": "Free software under the AGPL-3.0"},
}


def _e(x):
    return html.escape(x, quote=True)


def _lien(url, contenu, courant, classe=""):
    attrs = f' class="{classe}"' if classe else ""
    if url.startswith("http"):
        attrs += ' rel="noopener"'
    if url == courant:
        attrs += ' aria-current="page"'
    return f'<a href="{_e(url)}"{attrs}>{contenu}</a>'


def entete(langue="fr", courant="", langue_autre=None):
    """En-tête du site. courant = adresse de la page (son lien porte aria-current, sa rubrique
    est marquée) ; langue_autre = (code, adresse) pour le lien de changement de langue."""
    t = TEXTES[langue]
    groupes = []
    for i, (titre, liens) in enumerate(MENU[langue]):
        actif = any(u == courant or (u != "/" and not u.startswith(("http", "/#")) and courant.startswith(u.split("#")[0]))
                    for u, _, _ in liens)
        marque = ' data-actif' if actif else ""
        if len(liens) == 1:
            u, libelle, _ = liens[0]
            groupes.append(f'<div class="nav-groupe"{marque}>{_lien(u, _e(libelle), courant, "nav-titre")}</div>')
            continue
        entrees = "\n          ".join(
            _lien(u, f"<strong>{_e(l)}</strong><small>{_e(d)}</small>", courant) for u, l, d in liens)
        groupes.append(
            f'<div class="nav-groupe"{marque}>\n'
            f'        <button class="nav-titre" type="button" aria-expanded="false" aria-controls="nav-{i}">{_e(titre)}'
            f'<svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="M3 4.5 6 7.5 9 4.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></button>\n'
            f'        <div class="nav-panneau" id="nav-{i}">\n          {entrees}\n        </div>\n      </div>')
    autre = ""
    if langue_autre:
        code, url = langue_autre
        autre = f'\n      <a class="nav-langue" href="{_e(url)}" hreflang="{code}" lang="{code}">{code.upper()}</a>'
    return f"""<header class="entete">
  <div class="conteneur">
    <a class="marque" href="/" aria-label="{_e(t["accueil"])}"><img src="/assets/img/icone.svg" alt="" width="30" height="30">E-KO RIP Center</a>
    <button class="nav-bouton" aria-label="{t["menu"]}" aria-expanded="false" aria-controls="nav"><svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 7h16M4 12h16M4 17h16"/></svg></button>
    <nav class="nav" id="nav">
      {(chr(10) + "      ").join(groupes)}{autre}
      <a class="nav-mobile bouton bouton-plein" href="/telecharger/">{t["telecharger"]}</a>
    </nav>
    <a class="entete-contact" href="/contact/"{' aria-current="page"' if courant == "/contact/" else ""}>{t["contact"]}</a>
    <a class="bouton bouton-plein bouton-petit entete-cta" href="/telecharger/">{t["telecharger"]}</a>
  </div>
</header>"""


def pied(langue="fr"):
    t = TEXTES[langue]
    colonnes = "\n      ".join(
        f'<nav aria-label="{_e(titre)}"><h2>{_e(titre)}</h2>' + "".join(_lien(u, _e(l), "") for u, l in liens) + "</nav>"
        for titre, liens in t["colonnes"])
    return f"""<footer class="pied">
  <div class="conteneur pied-grille">
    <div class="pied-marque">
      <a class="marque" href="/"><img src="/assets/img/icone.svg" alt="" width="30" height="30">E-KO RIP Center</a>
      <p>{_e(t["slogan"])}</p>
      <p><a class="bouton bouton-plein bouton-petit" href="/telecharger/">{t["telecharger"]}</a> <a class="bouton bouton-clair bouton-petit" href="/contact/">{t["contact"]}</a></p>
    </div>
    {colonnes}
  </div>
  <div class="conteneur pied-bas">
    <span>© <span data-annee>2026</span> GOLDEN PRINT · E-KO RIP Center · {_e(t["droits"])}</span>
    <a href="mailto:contact@e-ko.rip">contact@e-ko.rip</a>
  </div>
</footer>"""
