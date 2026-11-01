#!/usr/bin/env python3
"""Génère la documentation publique (https://e-ko.rip/documentation/) depuis docs/.

    python3 outils/generer-documentation.py            # écrit site/documentation/
    python3 outils/generer-documentation.py --verifier # échoue si site/documentation/ n'est pas à jour

La source est en Markdown, en deux langues : les pages listées dans docs/guide/fr/sommaire.tsv
(publiées sous /documentation/) et docs/guide/en/sommaire.tsv (sous /documentation/en/) ;
pages du guide dans docs/guide/<langue>/, pages de référence reprises de docs/fr/ et docs/en/.
Les images de docs/guide/images/ sont copiées dans site/documentation/images/. Les pages générées sont commitées : le site
reste statique, sans étape de construction à la publication.

Aucune dépendance : un convertisseur Markdown réduit (titres, paragraphes, listes imbriquées,
tableaux, code, citations, encadrés [!NOTE] / [!TIP] / [!WARNING], images, liens) suffit au
Markdown du dépôt. Un lien vers un .md publié devient un lien vers sa page ; un lien vers un
autre fichier du dépôt part sur GitHub.
"""
import html
import json
import os
import re
import shutil
import sys
import tempfile
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
DOCS = RACINE / "docs"
IMAGES = DOCS / "guide" / "images"
SORTIE = RACINE / "site" / "documentation"
URL = "/documentation/"
LANGUES = {
    "fr": {"url": "/documentation/", "autre": "en", "nom": "Français",
           "t": {"doc": "Documentation", "evitement": "Aller au contenu", "menu": "Menu", "chercher": "Chercher…",
                 "chercher_long": "Chercher dans la documentation", "sommaire": "Sommaire de la documentation", "sommaire_court": "Sommaire",
                 "page": "Sur cette page", "prec": "Précédent", "suiv": "Suivant", "voisines": "Pages voisines",
                 "erreur": "Une erreur, un manque ?", "corriger": "Proposer une correction", "ecrire": "ou écrire à",
                 "nav": [("/#fonctionnalites", "Fonctionnalités"), ("/compatibilite/", "Machines"), ("/documentation/", "Documentation"),
                         ("/#installer", "Installer"), ("/changelog/", "Nouveautés")],
                 "installer": "Installer", "pied": [("/changelog/", "Nouveautés"), ("/prerequis/", "Pré-requis"), ("/app/", "Application mobile"),
                                                     ("/mentions-legales/", "Mentions légales")],
                 "aucun": "Aucun résultat.", "genres": {"NOTE": "À savoir", "TIP": "Astuce", "IMPORTANT": "Important", "WARNING": "Attention", "CAUTION": "Attention"}}},
    "en": {"url": "/documentation/en/", "autre": "fr", "nom": "English",
           "t": {"doc": "Documentation", "evitement": "Skip to content", "menu": "Menu", "chercher": "Search…",
                 "chercher_long": "Search the documentation", "sommaire": "Documentation contents", "sommaire_court": "Contents",
                 "page": "On this page", "prec": "Previous", "suiv": "Next", "voisines": "Neighbouring pages",
                 "erreur": "Found a mistake or something missing?", "corriger": "Suggest a fix", "ecrire": "or write to",
                 "nav": [("/#fonctionnalites", "Features"), ("/compatibilite/", "Machines"), ("/documentation/en/", "Documentation"),
                         ("/#installer", "Install"), ("/changelog/", "What's new")],
                 "installer": "Install", "pied": [("/changelog/", "What's new"), ("/prerequis/", "Requirements"), ("/app/", "Mobile app"),
                                                   ("/mentions-legales/", "Legal notice")],
                 "aucun": "No results.", "genres": {"NOTE": "Note", "TIP": "Tip", "IMPORTANT": "Important", "WARNING": "Warning", "CAUTION": "Caution"}}},
}
GITHUB = "https://github.com/mvignaud/eko-rip-center/blob/main/"


# ---------------------------------------------------------------- sommaire

def lire_sommaire(langue):
    """Rubriques et pages, dans l'ordre de docs/guide/<langue>/sommaire.tsv.

    Première colonne : identifiant de la page, commun aux deux langues (il relie une page à sa
    traduction), suivi au besoin de « :adresse » quand l'adresse diffère (« serveur:server »).
    """
    SOMMAIRE = DOCS / "guide" / langue / "sommaire.tsv"
    base = LANGUES[langue]["url"]
    pages, renvois, rubrique = [], {}, None
    for n, ligne in enumerate(SOMMAIRE.read_text(encoding="utf-8").splitlines(), 1):
        if not ligne.strip() or ligne.startswith("#"):
            continue
        if ligne.startswith("-> "):
            fichier, _, page = ligne[3:].partition("\t")
            source = (DOCS / fichier.strip()).resolve()
            if not source.is_file() or not page.strip():
                sys.exit(f"{SOMMAIRE}:{n} : renvoi « -> fichier<TAB>page » invalide")
            renvois[source] = page.strip()
            continue
        if ligne.startswith("== "):
            rubrique = ligne[3:].strip()
            continue
        champs = [c.strip() for c in ligne.split("\t") if c.strip()]
        if len(champs) != 4:
            sys.exit(f"{SOMMAIRE}:{n} : attendu « page<TAB>fichier<TAB>titre<TAB>description »")
        page, fichier, titre, description = champs
        ident, _, adresse = page.partition(":")
        source = (DOCS / fichier).resolve()
        if not source.is_file():
            sys.exit(f"{SOMMAIRE}:{n} : {fichier} introuvable dans docs/")
        adresse = adresse or ident
        url = base if adresse == "index" else f"{base}{adresse}/"
        pages.append({"id": ident, "page": adresse, "url": url, "source": source, "titre": titre,
                      "description": description, "rubrique": rubrique, "langue": langue})
    par_id = {p["id"]: p["url"] for p in pages}
    for source, page in list(renvois.items()):
        if page not in par_id:
            sys.exit(f"{SOMMAIRE} : renvoi vers une page inconnue « {page} »")
        renvois[source] = par_id[page]
    return pages, renvois


# ---------------------------------------------------------------- Markdown

def ancre(texte, deja):
    """Ancre à la GitHub : minuscules, ponctuation retirée, espaces en tirets, accents gardés."""
    t = re.sub(r"<[^>]+>", "", texte)
    t = html.unescape(t).strip().lower()
    t = "".join(c for c in t if c.isalnum() or c in " -_")
    t = t.replace(" ", "-")
    base, i = t, 1
    while t in deja:
        t = f"{base}-{i}"
        i += 1
    deja.add(t)
    return t


class Convertisseur:
    def __init__(self, source, liens, renvois, images_utilisees, genres):
        self.genres = genres
        self.source = source            # chemin du .md converti
        self.liens = liens              # chemin .md résolu -> URL de page
        self.renvois = renvois          # chemin .md résolu -> URL de page, ancre abandonnée
        self.images = images_utilisees  # noms d'images référencées
        self.ancres = set()
        self.titres = []                # (niveau, texte, ancre) pour « Sur cette page »

    # -- liens et images
    def cible(self, url):
        if re.match(r"^(https?:|mailto:|#|/)", url):
            return url
        chemin, _, fragment = url.partition("#")
        fichier = (self.source.parent / chemin).resolve()
        if fichier.suffix == ".md" and fichier in self.liens:
            return self.liens[fichier] + (f"#{fragment}" if fragment else "")
        if fichier in self.renvois:
            return self.renvois[fichier]
        if fichier == self.source and not chemin:
            return f"#{fragment}"
        try:
            relatif = fichier.relative_to(RACINE)
        except ValueError:
            return url
        return GITHUB + relatif.as_posix() + (f"#{fragment}" if fragment else "")

    def image(self, src):
        if re.match(r"^(https?:|/)", src):
            return src
        fichier = (self.source.parent / src).resolve()
        if fichier.parent != IMAGES.resolve():
            sys.exit(f"{self.source} : l'image {src} doit être dans docs/guide/images/")
        if not fichier.is_file():
            sys.exit(f"{self.source} : image introuvable {src}")
        self.images.add(fichier.name)
        return f"{URL}images/{fichier.name}"

    # -- en ligne
    def en_ligne(self, texte):
        morceaux = re.split(r"(`+)(.+?)\1", texte)
        sortie = []
        for i in range(0, len(morceaux), 3):
            sortie.append(self.en_ligne_sans_code(morceaux[i]))
            if i + 2 < len(morceaux):
                sortie.append(f"<code>{html.escape(morceaux[i + 2].strip(), quote=False)}</code>")
        return "".join(sortie)

    def en_ligne_sans_code(self, t):
        gardes = []

        def garder(fragment):
            gardes.append(fragment)
            return f"\x00{len(gardes) - 1}\x00"

        t = re.sub(r"<(https?://[^>\s]+)>", lambda m: garder(f'<a href="{html.escape(m[1])}">{html.escape(m[1])}</a>'), t)
        t = re.sub(r"<br\s*/?>", lambda m: garder("<br>"), t)
        t = re.sub(r"<kbd>(.*?)</kbd>", lambda m: garder(f"<kbd>{html.escape(m[1])}</kbd>"), t)

        def img(m):
            alt, src = m[1], m[2]
            return garder(f'<img src="{html.escape(self.image(src))}" alt="{html.escape(alt)}" loading="lazy">')

        t = re.sub(r"!\[([^\]]*)\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)", img, t)

        def lien(m):
            texte, url = m[1], self.cible(m[2])
            externe = ' rel="noopener"' if url.startswith("http") else ""
            return garder(f'<a href="{html.escape(url)}"{externe}>') + texte + garder("</a>")

        t = re.sub(r"\[([^\]]+)\]\(([^)\s]+)\)", lien, t)
        t = html.escape(t, quote=False)
        t = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", t)
        t = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", t)
        t = re.sub(r"(?<![\w])_(?!\s)([^_]+?)(?<!\s)_(?![\w])", r"<em>\1</em>", t)
        if self.genres["TIP"] == "Astuce":  # typographie française : espace fine insécable
            t = re.sub(r" ([:;?!])", "\u202f\\1", t)
        return re.sub(r"\x00(\d+)\x00", lambda m: gardes[int(m[1])], t)

    # -- blocs
    def blocs(self, lignes):
        sortie, i = [], 0
        while i < len(lignes):
            ligne = lignes[i]
            if not ligne.strip():
                i += 1
                continue
            m = re.match(r"^\s*(```+|~~~+)\s*([\w-]*)", ligne)
            if m:
                fin, langue, j = m[1], m[2], i + 1
                code = []
                while j < len(lignes) and not lignes[j].strip().startswith(fin):
                    code.append(lignes[j])
                    j += 1
                retrait = min((len(c) - len(c.lstrip()) for c in code if c.strip()), default=0)
                corps = html.escape("\n".join(c[retrait:] for c in code), quote=False)
                classe = f' class="langue-{langue}"' if langue else ""
                sortie.append(f"<pre{classe}><code>{corps}</code></pre>")
                i = j + 1
                continue
            m = re.match(r"^(#{1,6})\s+(.*?)\s*#*\s*$", ligne)
            if m:
                niveau = len(m[1])
                contenu = self.en_ligne(m[2])
                if niveau == 1:  # le titre de la page vient du sommaire
                    i += 1
                    continue
                id_ = ancre(contenu, self.ancres)
                if niveau <= 3:
                    self.titres.append((niveau, re.sub(r"<[^>]+>", "", contenu), id_))
                sortie.append(f'<h{niveau} id="{id_}"><a class="ancre" href="#{id_}" aria-hidden="true">#</a>{contenu}</h{niveau}>')
                i += 1
                continue
            if re.match(r"^\s*([-*_])(\s*\1){2,}\s*$", ligne):
                sortie.append("<hr>")
                i += 1
                continue
            if ligne.lstrip().startswith("|") and i + 1 < len(lignes) and re.match(r"^\s*\|?\s*:?-{2,}", lignes[i + 1]):
                j = i + 2
                while j < len(lignes) and lignes[j].lstrip().startswith("|"):
                    j += 1
                sortie.append(self.tableau(lignes[i:j]))
                i = j
                continue
            if ligne.lstrip().startswith(">"):
                j = i
                while j < len(lignes) and lignes[j].lstrip().startswith(">"):
                    j += 1
                sortie.append(self.citation([re.sub(r"^\s*>\s?", "", l) for l in lignes[i:j]]))
                i = j
                continue
            if re.match(r"^\s*([-*+]|\d+[.)])\s+", ligne):
                j = self.fin_de_liste(lignes, i)
                sortie.append(self.liste(lignes[i:j]))
                i = j
                continue
            if re.match(r"^\s*<(figure|div|details|table|p|svg|aside)\b", ligne):
                j = i
                while j < len(lignes) and lignes[j].strip():
                    j += 1
                sortie.append("\n".join(lignes[i:j]))
                i = j
                continue
            j = i
            while j < len(lignes) and lignes[j].strip() and not self.debut_de_bloc(lignes[j]):
                j += 1
            j = max(j, i + 1)
            sortie.append(self.paragraphe(lignes[i:j]))
            i = j
        return "\n".join(sortie)

    @staticmethod
    def debut_de_bloc(ligne):
        return bool(re.match(r"^\s*(```|~~~|#{1,6}\s|>|([-*+]|\d+[.)])\s+|\|)", ligne))

    def paragraphe(self, lignes):
        texte = " ".join(l.strip() for l in lignes)
        seule = re.fullmatch(r"!\[([^\]]*)\]\(([^)\s]+)(?:\s+\"([^\"]*)\")?\)", texte)
        if seule:
            alt, src, legende = seule[1], seule[2], seule[3] or seule[1]
            img = f'<img src="{html.escape(self.image(src))}" alt="{html.escape(alt)}" loading="lazy">'
            lien = html.escape(self.image(src))  # un clic ouvre l'image en grand (schémas sur téléphone)
            return f'<figure><a href="{lien}">{img}</a><figcaption>{self.en_ligne(legende)}</figcaption></figure>'
        return f"<p>{self.en_ligne(texte)}</p>"

    def tableau(self, lignes):
        def cellules(l):
            l = l.strip()
            l = l[1:] if l.startswith("|") else l
            l = l[:-1] if l.endswith("|") and not l.endswith("\\|") else l
            return [c.strip().replace("\\|", "|") for c in re.split(r"(?<!\\)\|", l)]

        alignements = []
        for c in cellules(lignes[1]):
            alignements.append("center" if c.startswith(":") and c.endswith(":") else "right" if c.endswith(":") else "")

        def rangee(l, balise):
            cs = cellules(l)
            out = []
            for k, c in enumerate(cs):
                a = alignements[k] if k < len(alignements) else ""
                style = f' style="text-align:{a}"' if a else ""
                out.append(f"<{balise}{style}>{self.en_ligne(c)}</{balise}>")
            return "<tr>" + "".join(out) + "</tr>"

        corps = "".join(rangee(l, "td") for l in lignes[2:])
        return f'<div class="tableau"><table><thead>{rangee(lignes[0], "th")}</thead><tbody>{corps}</tbody></table></div>'

    def citation(self, lignes):
        m = re.match(r"^\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION)\]\s*$", lignes[0]) if lignes else None
        if m:
            classe = {"NOTE": "note", "TIP": "astuce", "IMPORTANT": "important", "WARNING": "attention", "CAUTION": "attention"}[m[1]]
            return f'<aside class="encadre encadre-{classe}"><p class="encadre-titre">{self.genres[m[1]]}</p>{self.blocs(lignes[1:])}</aside>'
        return f"<blockquote>{self.blocs(lignes)}</blockquote>"

    @staticmethod
    def retrait(l):
        return len(l) - len(l.lstrip(" "))

    def fin_de_liste(self, lignes, i):
        base = self.retrait(lignes[i])
        j = i + 1
        while j < len(lignes):
            l = lignes[j]
            if not l.strip():
                k = j + 1
                while k < len(lignes) and not lignes[k].strip():
                    k += 1
                if k < len(lignes) and (self.retrait(lignes[k]) > base or re.match(r"^\s*([-*+]|\d+[.)])\s+", lignes[k]) and self.retrait(lignes[k]) == base):
                    j = k
                    continue
                return j
            if self.retrait(l) <= base and not re.match(r"^\s*([-*+]|\d+[.)])\s+", l) and re.match(r"^\s*(```|#{1,6}\s|>|\|)", l):
                return j
            j += 1
        return j

    def liste(self, lignes):
        base = self.retrait(lignes[0])
        ordonnee = bool(re.match(r"^\s*\d+[.)]\s", lignes[0]))
        items, courant, decalage = [], None, 0
        for l in lignes:
            m = re.match(r"^(\s*)([-*+]|\d+[.)])(\s+)(.*)$", l)
            if m and len(m[1]) == base:
                courant = [m[4]]
                decalage = len(m[1]) + len(m[2]) + len(m[3])
                items.append(courant)
            elif courant is not None:
                courant.append(l[min(decalage, self.retrait(l)):] if l.strip() else "")
        rendus = []
        for item in items:
            while item and not item[-1].strip():
                item.pop()
            blocs = self.blocs(item)
            if "\n" not in blocs and blocs.startswith("<p>") and blocs.endswith("</p>"):
                blocs = blocs[3:-4]
            elif "" not in item:  # liste serrée : pas de <p> autour du premier paragraphe
                blocs = re.sub(r"^<p>(.*?)</p>", r"\1", blocs, count=1, flags=re.S)
            rendus.append(f"<li>{blocs}</li>")
        balise = "ol" if ordonnee else "ul"
        depart = re.match(r"^\s*(\d+)", lignes[0])
        attribut = f' start="{depart[1]}"' if ordonnee and depart and depart[1] != "1" else ""
        return f"<{balise}{attribut}>" + "".join(rendus) + f"</{balise}>"


# ---------------------------------------------------------------- gabarit

def gabarit(p, contenu, titres, pages, precedente, suivante, traduction):
    L = LANGUES[p["langue"]]
    t = L["t"]
    nav, rubrique = [], None
    for q in pages:
        if q["rubrique"] != rubrique:
            if rubrique is not None:
                nav.append("</ul>")
            rubrique = q["rubrique"]
            nav.append(f'<p class="doc-rubrique">{html.escape(rubrique or "")}</p><ul>')
        actuelle = ' aria-current="page"' if q is p else ""
        nav.append(f'<li><a href="{q["url"]}"{actuelle}>{html.escape(q["titre"])}</a></li>')
    nav.append("</ul>")
    toc = "".join(f'<li class="niveau-{n}"><a href="#{a}">{html.escape(html.unescape(x))}</a></li>' for n, x, a in titres)
    toc_bloc = f'<nav class="doc-toc" aria-label="{t["page"]}"><p>{t["page"]}</p><ul>{toc}</ul></nav>' if titres else ""
    suite = []
    if precedente:
        suite.append(f'<a class="doc-prec" href="{precedente["url"]}"><span>{t["prec"]}</span>{html.escape(precedente["titre"])}</a>')
    if suivante:
        suite.append(f'<a class="doc-suiv" href="{suivante["url"]}"><span>{t["suiv"]}</span>{html.escape(suivante["titre"])}</a>')
    source = p["source"].relative_to(RACINE).as_posix()
    titre_onglet = t["doc"] if p["page"] == "index" else f'{p["titre"]} · {t["doc"]}'
    autre = LANGUES[L["autre"]]
    lien_autre = traduction["url"] if traduction else autre["url"]
    hreflang = f'<link rel="alternate" hreflang="{p["langue"]}" href="https://e-ko.rip{p["url"]}">'
    if traduction:
        hreflang += f'\n<link rel="alternate" hreflang="{L["autre"]}" href="https://e-ko.rip{traduction["url"]}">'
    courant = ' aria-current="page"'
    nav_site = "\n      ".join(f'<a href="{u}"{courant if u == L["url"] else ""}>{html.escape(n)}</a>' for u, n in t["nav"])
    pied = "\n      ".join(f'<a href="{u}">{html.escape(n)}</a>' for u, n in t["pied"])
    fil = "" if p["page"] == "index" else " › " + html.escape(p["rubrique"] or "")
    return f"""<!doctype html>
<html lang="{p["langue"]}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(titre_onglet)} · E-KO RIP Center</title>
<meta name="description" content="{html.escape(p["description"])}">
<link rel="canonical" href="https://e-ko.rip{p["url"]}">
{hreflang}
<link rel="icon" href="/assets/img/icone.svg" type="image/svg+xml">
<meta name="theme-color" content="#0a84ff">
<link rel="stylesheet" href="/assets/style.css">
<link rel="stylesheet" href="/assets/documentation.css">
</head>
<body class="doc" data-doc-langue="{p["langue"]}">
<a class="lien-evitement" href="#contenu">{t["evitement"]}</a>
<!-- Page générée par outils/generer-documentation.py depuis {source} : ne pas modifier ici. -->

<header class="entete">
  <div class="conteneur">
    <a class="marque" href="/"><img src="/assets/img/icone.svg" alt="" width="30" height="30">E-KO RIP Center</a>
    <button class="nav-bouton" aria-label="{t["menu"]}" aria-expanded="false" aria-controls="nav"><svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M4 7h16M4 12h16M4 17h16"/></svg></button>
    <nav class="nav" id="nav">
      {nav_site}
      <a href="{lien_autre}" hreflang="{L["autre"]}" lang="{L["autre"]}" class="doc-langue">{L["autre"].upper()}</a>
    </nav>
    <a class="bouton bouton-plein bouton-petit" href="/#installer">{t["installer"]}</a>
  </div>
</header>

<div class="conteneur doc-grille">
  <aside class="doc-menu" aria-label="{t["sommaire"]}">
    <button class="doc-menu-bouton" type="button" aria-expanded="false" data-doc-menu>{t["sommaire_court"]}</button>
    <label class="doc-filtre"><span class="visuellement-cache">{t["chercher_long"]}</span><input type="search" placeholder="{t["chercher"]}" data-doc-filtre data-doc-aucun="{t["aucun"]}"></label>
    <nav data-doc-nav>{"".join(nav)}</nav>
    <ul class="doc-resultats" data-doc-resultats hidden></ul>
  </aside>
  <main id="contenu" class="doc-article">
    <p class="doc-fil"><a href="{L["url"]}">{t["doc"]}</a>{fil}</p>
    <h1>{html.escape(p["titre"])}</h1>
    <p class="doc-chapo">{html.escape(p["description"])}</p>
{contenu}
    <nav class="doc-suite" aria-label="{t["voisines"]}">{"".join(suite)}</nav>
    <p class="doc-source">{t["erreur"]} <a href="https://github.com/mvignaud/eko-rip-center/edit/main/{source}" rel="noopener">{t["corriger"]}</a> {t["ecrire"]} <a href="mailto:contact@e-ko.rip">contact@e-ko.rip</a>.</p>
  </main>
  {toc_bloc}
</div>

<footer class="pied">
  <div class="conteneur">
    <span>© <span data-annee>2026</span> GOLDEN PRINT · E-KO RIP Center</span>
    <nav>
      <a href="{L["url"]}">{t["doc"]}</a>
      {pied}
      <a href="https://platform.e-ko.rip">Cloud E-KO RIP</a>
      <a href="https://github.com/mvignaud/eko-rip-center" rel="noopener">GitHub</a>
      <a href="mailto:contact@e-ko.rip">contact@e-ko.rip</a>
    </nav>
  </div>
</footer>

<script src="/assets/site.js" defer></script>
<script src="/assets/documentation.js" defer></script>
</body>
</html>
"""


def texte_brut(fragment):
    x = re.sub(r"<pre.*?</pre>", " ", fragment, flags=re.S)
    x = re.sub(r"<[^>]+>", " ", x)
    return re.sub(r"\s+", " ", html.unescape(x)).strip()


def generer(dossier):
    """Écrit les deux langues dans dossier (= site/documentation/) ; rend le nombre de pages."""
    sommaires = {langue: lire_sommaire(langue) for langue in LANGUES}
    par_id = {langue: {p["id"]: p for p in pages} for langue, (pages, _) in sommaires.items()}
    images, total = set(), 0
    for langue, (pages, renvois) in sommaires.items():
        L = LANGUES[langue]
        liens = {p["source"]: p["url"] for p in pages}
        index_recherche = []
        for k, p in enumerate(pages):
            conv = Convertisseur(p["source"], liens, renvois, images, L["t"]["genres"])
            contenu = conv.blocs(p["source"].read_text(encoding="utf-8").splitlines())
            relatif = p["url"][len(URL):]
            sortie = dossier / relatif / "index.html"
            sortie.parent.mkdir(parents=True, exist_ok=True)
            prec = pages[k - 1] if k > 0 else None
            suiv = pages[k + 1] if k + 1 < len(pages) else None
            traduction = par_id[L["autre"]].get(p["id"])
            sortie.write_text(gabarit(p, contenu, conv.titres, pages, prec, suiv, traduction), encoding="utf-8")
            # Index de recherche : une entrée par section de niveau 2 ou 3.
            for bloc in re.split(r'(?=<h[23] id=")', contenu):
                m = re.match(r'<h[23] id="([^"]+)">(.*?)</h[23]>', bloc, flags=re.S)
                titre = texte_brut(m[2]).lstrip("#").strip() if m else p["titre"]
                corps = texte_brut(bloc[m.end():] if m else bloc)
                if corps or m:
                    index_recherche.append({"p": p["titre"], "t": titre, "u": p["url"] + (f"#{m[1]}" if m else ""), "x": corps[:600]})
            total += 1
        cible = dossier / L["url"][len(URL):] / "recherche.json"
        cible.parent.mkdir(parents=True, exist_ok=True)
        cible.write_text(json.dumps(index_recherche, ensure_ascii=False, separators=(",", ":")) + "\n", encoding="utf-8")
    urls = "".join(f"  <url><loc>https://e-ko.rip{p['url']}</loc></url>\n" for pages, _ in sommaires.values() for p in pages)
    (dossier / "sitemap.xml").write_text('<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n' + urls + "</urlset>\n", encoding="utf-8")
    if images:
        (dossier / "images").mkdir(exist_ok=True)
        for nom in sorted(images):
            shutil.copyfile(IMAGES / nom, dossier / "images" / nom)
    orphelines = sorted({f.name for f in IMAGES.iterdir() if f.is_file()} - images) if IMAGES.is_dir() else []
    for nom in orphelines:
        print(f"Avertissement : image non utilisée docs/guide/images/{nom}", file=sys.stderr)
    return total


def fichiers(dossier):
    return {f.relative_to(dossier).as_posix(): f.read_bytes() for f in dossier.rglob("*") if f.is_file()}


def main():
    if "--verifier" in sys.argv:
        with tempfile.TemporaryDirectory() as tmp:
            generer(Path(tmp))
            attendu, present = fichiers(Path(tmp)), fichiers(SORTIE) if SORTIE.is_dir() else {}
        ecarts = sorted(n for n in set(attendu) | set(present) if attendu.get(n) != present.get(n))
        if ecarts:
            print("site/documentation/ n'est pas à jour avec docs/ : lancer python3 outils/generer-documentation.py", file=sys.stderr)
            for n in ecarts[:20]:
                print(f"  {n}", file=sys.stderr)
            sys.exit(1)
        print("Documentation à jour.")
        return
    if SORTIE.exists():
        shutil.rmtree(SORTIE)
    SORTIE.mkdir(parents=True)
    n = generer(SORTIE)
    print(f"{n} pages écrites dans {SORTIE.relative_to(RACINE)}/")


if __name__ == "__main__":
    main()
