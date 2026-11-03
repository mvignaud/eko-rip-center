#!/usr/bin/env python3
"""Génère les parties construites du site https://e-ko.rip (dossier site/).

    python3 outils/generer-site.py            # écrit le blog, les pages, le plan du site
    python3 outils/generer-site.py --verifier # échoue si site/ n'est pas à jour

Ce qu'il écrit :
- le menu et le pied communs (outils/site_commun.py) dans chaque page écrite à la main, entre
  <!-- entete --> et <!-- /entete -->, <!-- pied --> et <!-- /pied --> ;
- le blog : blog/fr/*.md → site/blog/<adresse>/, l'index, les catégories, le flux RSS ;
  les trois derniers articles sur l'accueil (entre <!-- blog-recents --> et <!-- /blog-recents -->) ;
- les pages en Markdown : pages/*.md → site/<nom>/ (feuille de route) ;
- site/sitemap.xml et site/llms.txt.

Aucune dépendance : le convertisseur Markdown est celui de outils/generer-documentation.py.
Les pages générées sont commitées : le site reste statique, sans étape de construction.
"""
import html
import importlib.util
import json
import re
import sys
from datetime import date, datetime, timezone
from email.utils import format_datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import site_commun  # noqa: E402

RACINE = Path(__file__).resolve().parent.parent
SITE = RACINE / "site"
BLOG = RACINE / "blog" / "fr"
PAGES = RACINE / "pages"
URL = "https://e-ko.rip"

_spec = importlib.util.spec_from_file_location("generer_documentation", Path(__file__).resolve().parent / "generer-documentation.py")
_doc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_doc)
GENRES = _doc.LANGUES["fr"]["t"]["genres"]

CATEGORIES = {"Guides": "guides", "Couleur": "couleur", "Atelier": "atelier", "Logiciel": "logiciel", "Comparatifs": "comparatifs"}
DESCRIPTIONS_CAT = {
    "Guides": "Comprendre et choisir un RIP grand format : les bases, les critères, la méthode.",
    "Couleur": "Profils ICC, blanc, argent, étalonnage du gris et linéarisation : la couleur maîtrisée.",
    "Atelier": "Impression, découpe, repérage et pilotage à distance : le quotidien de la production.",
    "Logiciel": "Open source, ERP, intégrations : ce qu'il y a sous le capot d'E-KO RIP Center.",
    "Comparatifs": "E-KO RIP Center face aux autres RIP du marché, d'après les informations publiques des éditeurs.",
}
MOIS = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"]
COLONNES_FDR = ["Fait", "En cours", "Planifié"]
SOURCE_PAGES = {"index.html", "404.html"}  # pages écrites à la main : tout index.html hors dossiers générés
DOSSIERS_GENERES = ("documentation/", "blog/", "feuille-de-route/")


def e(x):
    return html.escape(str(x), quote=True)


def date_fr(d):
    return f"{d.day} {MOIS[d.month - 1]} {d.year}"


def erreur(msg):
    sys.exit(f"generer-site : {msg}")


def taille_image(chemin_web):
    """(largeur, hauteur) réelles d'une image WebP de site/ (en-tête lu sans dépendance)."""
    data = (SITE / chemin_web.lstrip("/")).read_bytes()[:40]
    if data[:4] != b"RIFF" or data[8:12] != b"WEBP":
        erreur(f"{chemin_web} : seules les images WebP sont prises en charge")
    bloc = data[12:16]
    if bloc == b"VP8X":
        return 1 + int.from_bytes(data[24:27], "little"), 1 + int.from_bytes(data[27:30], "little")
    if bloc == b"VP8L":
        b = int.from_bytes(data[21:25], "little")
        return (b & 0x3FFF) + 1, ((b >> 14) & 0x3FFF) + 1
    if bloc == b"VP8 ":
        return int.from_bytes(data[26:28], "little") & 0x3FFF, int.from_bytes(data[28:30], "little") & 0x3FFF
    erreur(f"{chemin_web} : en-tête WebP inconnu")


def img(src, alt="", charge=""):
    l, h = taille_image(src)
    paresseux = ' loading="lazy"' if charge == "lazy" else ""
    return f'<img src="{e(src)}" alt="{e(alt)}"{paresseux} width="{l}" height="{h}">'


# ---------------------------------------------------------------- lecture

def lire_markdown(chemin, obligatoires):
    texte = chemin.read_text(encoding="utf-8")
    m = re.match(r"^---\n(.*?)\n---\n(.*)$", texte, flags=re.S)
    if not m:
        erreur(f"{chemin.relative_to(RACINE)} : en-tête --- manquant")
    meta = {}
    for ligne in m[1].splitlines():
        if ligne.strip():
            cle, _, val = ligne.partition(":")
            meta[cle.strip()] = val.strip()
    manquants = [c for c in obligatoires if not meta.get(c)]
    if manquants:
        erreur(f"{chemin.relative_to(RACINE)} : champ(s) manquant(s) {', '.join(manquants)}")
    return meta, m[2]


def convertir(source, corps):
    conv = _doc.Convertisseur(source, {}, {}, set(), GENRES)
    contenu = conv.blocs(corps.splitlines())
    return contenu, conv


def faq(corps):
    """Questions de la section « ## Questions fréquentes » : [(question, réponse en texte)]."""
    m = re.search(r"^## Questions fréquentes\s*$(.*?)(?=^## |\Z)", corps, flags=re.M | re.S)
    if not m:
        return []
    paires = re.findall(r"^### (.+?)\s*$(.*?)(?=^### |\Z)", m[1], flags=re.M | re.S)
    nettoyer = lambda t: re.sub(r"\s+", " ", re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", re.sub(r"[*_`]", "", t))).strip()
    return [(nettoyer(q), nettoyer(r)) for q, r in paires if r.strip()]


def lire_articles():
    articles = []
    for f in sorted(BLOG.glob("*.md")):
        meta, corps = lire_markdown(f, ["titre", "description", "date", "categorie"])
        if meta["categorie"] not in CATEGORIES:
            erreur(f"{f.relative_to(RACINE)} : catégorie inconnue « {meta['categorie']} » ({', '.join(CATEGORIES)})")
        if not re.fullmatch(r"[a-z0-9]+(-[a-z0-9]+)*", f.stem):
            erreur(f"{f.relative_to(RACINE)} : nom de fichier = adresse, en minuscules et tirets")
        try:
            d = date.fromisoformat(meta["date"])
        except ValueError:
            erreur(f"{f.relative_to(RACINE)} : date invalide {meta['date']}")
        if meta.get("image") and not (SITE / meta["image"].lstrip("/")).is_file():
            erreur(f"{f.relative_to(RACINE)} : image introuvable {meta['image']}")
        contenu, conv = convertir(f, corps)
        mots = len(re.findall(r"\w+", corps))
        articles.append({
            "slug": f.stem, "url": f"/blog/{f.stem}/", "source": f, "titre": meta["titre"], "description": meta["description"],
            "date": d, "maj": date.fromisoformat(meta["maj"]) if meta.get("maj") else d, "categorie": meta["categorie"],
            "image": meta.get("image", ""), "alt": meta.get("alt", ""), "contenu": contenu, "titres": conv.titres,
            "lecture": max(1, round(mots / 220)), "faq": faq(corps), "corps": corps,
        })
    articles.sort(key=lambda a: (a["date"], a["titre"]), reverse=True)
    return articles


# ---------------------------------------------------------------- gabarits

def tete(titre, description, chemin, extra="", image="/assets/img/linux-travaux.webp", type_og="website", prose=False):
    return f"""<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{e(titre)}</title>
<meta name="description" content="{e(description)}">
<link rel="canonical" href="{URL}{chemin}">
<link rel="icon" href="/favicon.ico" sizes="48x48">
<link rel="icon" href="/assets/img/icone.svg" type="image/svg+xml">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="alternate" type="application/rss+xml" title="Blog E-KO RIP Center" href="/blog/rss.xml">
<meta name="theme-color" content="#0a84ff">
<meta property="og:type" content="{type_og}">
<meta property="og:site_name" content="E-KO RIP Center">
<meta property="og:locale" content="fr_FR">
<meta property="og:title" content="{e(titre)}">
<meta property="og:description" content="{e(description)}">
<meta property="og:url" content="{URL}{chemin}">
<meta property="og:image" content="{URL}{e(image)}">
<meta name="twitter:card" content="summary_large_image">
<link rel="stylesheet" href="/assets/style.css">
{'<link rel="stylesheet" href="/assets/documentation.css">' + chr(10) if prose else ""}{extra}</head>
<body>
<a class="lien-evitement" href="#contenu">Aller au contenu</a>
<!-- Page générée par outils/generer-site.py : ne pas modifier ici. -->

{site_commun.entete("fr", chemin)}
"""


def queue():
    return f"""
{site_commun.pied("fr")}

<script src="/assets/site.js" defer></script>
</body>
</html>
"""


def ld(*objets):
    return "".join(f'<script type="application/ld+json">{json.dumps(o, ensure_ascii=False, separators=(",", ":"))}</script>\n' for o in objets)


def fil_ariane(elements):
    """elements = [(adresse, nom)] ; rend le HTML et l'objet BreadcrumbList."""
    liens = " › ".join(f'<a href="{u}">{e(n)}</a>' if i < len(elements) - 1 else f'<span aria-current="page">{e(n)}</span>'
                       for i, (u, n) in enumerate(elements))
    objet = {"@context": "https://schema.org", "@type": "BreadcrumbList", "itemListElement": [
        {"@type": "ListItem", "position": i + 1, "name": n, "item": URL + u} for i, (u, n) in enumerate(elements)]}
    return f'<nav class="fil" aria-label="Fil d\'Ariane">{liens}</nav>', objet


def carte(a, titre_niveau="h3"):
    image = (img(a["image"], charge="lazy") if a["image"]
             else '<span class="carte-billet-vide" aria-hidden="true"></span>')
    return f"""<article class="carte-billet apparait">
        <a href="{a["url"]}" class="carte-billet-image" tabindex="-1" aria-hidden="true">{image}</a>
        <div class="carte-billet-corps">
          <p class="carte-billet-meta"><span class="pastille-cat cat-{CATEGORIES[a["categorie"]]}">{e(a["categorie"])}</span> <time datetime="{a["date"].isoformat()}">{date_fr(a["date"])}</time> · {a["lecture"]} min</p>
          <{titre_niveau}><a href="{a["url"]}">{e(a["titre"])}</a></{titre_niveau}>
          <p>{e(a["description"])}</p>
        </div>
      </article>"""


def appel_final():
    return """<section class="appel-final">
  <div class="conteneur">
    <div class="appel-final-cadre apparait">
      <div>
        <h2>Essayez E-KO RIP Center dans votre atelier.</h2>
        <p>Gratuit, open source, installé en quelques minutes sur un serveur Ubuntu ou Debian. Une question sur votre machine ou votre flux ? Nous répondons nous-mêmes.</p>
      </div>
      <div class="actions">
        <a class="bouton bouton-plein" href="/telecharger/">Télécharger gratuitement</a>
        <a class="bouton bouton-clair" href="/contact/">Nous contacter</a>
      </div>
    </div>
  </div>
</section>"""


def page_article(a, articles):
    cat = a["categorie"]
    fil, fil_ld = fil_ariane([("/", "Accueil"), ("/blog/", "Blog"), (f"/blog/categorie/{CATEGORIES[cat]}/", cat), (a["url"], a["titre"])])
    toc = "".join(f'<li><a href="#{ancre}">{html.escape(html.unescape(x))}</a></li>' for n, x, ancre in a["titres"] if n == 2)
    voisins = [b for b in articles if b is not a and b["categorie"] == cat]
    voisins += [b for b in articles if b is not a and b not in voisins]
    billet = {
        "@context": "https://schema.org", "@type": "BlogPosting", "headline": a["titre"], "description": a["description"],
        "datePublished": a["date"].isoformat(), "dateModified": a["maj"].isoformat(), "inLanguage": "fr-FR",
        "mainEntityOfPage": URL + a["url"], "articleSection": cat, "wordCount": len(re.findall(r"\w+", a["corps"])),
        "author": {"@type": "Organization", "name": "E-KO RIP Center", "url": URL + "/"},
        "publisher": {"@type": "Organization", "name": "GOLDEN PRINT", "url": URL + "/", "logo": {"@type": "ImageObject", "url": URL + "/assets/img/icone-512.png"}},
    }
    if a["image"]:
        billet["image"] = URL + a["image"]
    objets = [billet, fil_ld]
    if a["faq"]:
        objets.append({"@context": "https://schema.org", "@type": "FAQPage", "mainEntity": [
            {"@type": "Question", "name": q, "acceptedAnswer": {"@type": "Answer", "text": r}} for q, r in a["faq"]]})
    maj = (f' · mis à jour le <time datetime="{a["maj"].isoformat()}">{date_fr(a["maj"])}</time>' if a["maj"] != a["date"] else "")
    image = (f'<figure class="billet-image">{img(a["image"], a["alt"])}</figure>' if a["image"] else "")
    return tete(f'{a["titre"]} · Blog E-KO RIP', a["description"], a["url"],
                extra=ld(*objets) + f'<meta property="article:published_time" content="{a["date"].isoformat()}">\n',
                image=a["image"] or "/assets/img/linux-travaux.webp", type_og="article", prose=True) + f"""
<main id="contenu">
<article class="billet">
  <header class="billet-tete">
    <div class="conteneur">
      {fil}
      <p><a class="pastille-cat cat-{CATEGORIES[cat]}" href="/blog/categorie/{CATEGORIES[cat]}/">{e(cat)}</a></p>
      <h1>{e(a["titre"])}</h1>
      <p class="billet-meta">Par l'équipe E-KO RIP · <time datetime="{a["date"].isoformat()}">{date_fr(a["date"])}</time>{maj} · {a["lecture"]} min de lecture</p>
      {image}
    </div>
  </header>
  <div class="conteneur billet-grille">
    <div class="billet-corps doc-article">
{a["contenu"]}
    </div>
    <aside class="billet-cote">
      {f'<nav class="billet-toc" aria-label="Dans cet article"><p>Dans cet article</p><ol>{toc}</ol></nav>' if toc else ""}
      <div class="billet-cta">
        <p><strong>E-KO RIP Center</strong></p>
        <p>Le RIP grand format libre et gratuit qui pilote tout l'atelier.</p>
        <a class="bouton bouton-plein bouton-petit" href="/telecharger/">Télécharger</a>
        <a class="bouton bouton-clair bouton-petit" href="/contact/">Poser une question</a>
      </div>
    </aside>
  </div>
</article>

<section class="bande">
  <div class="conteneur">
    <div class="section-titre apparait"><span class="surtitre">À lire aussi</span><h2>Dans la même veine</h2></div>
    <div class="grille-billets">
      {(chr(10) + "      ").join(carte(b) for b in voisins[:3])}
    </div>
  </div>
</section>

{appel_final()}
</main>
""" + queue()


def liste_billets(articles, tous, titre_page, chapo, chemin, surtitre, cat_active=None):
    filtres = ['<a href="/blog/"' + (' aria-current="page"' if cat_active is None else "") + ">Tous</a>"]
    for nom, slug in CATEGORIES.items():
        if any(a["categorie"] == nom for a in tous):
            courant = ' aria-current="page"' if cat_active == nom else ""
            filtres.append(f'<a href="/blog/categorie/{slug}/"{courant}>{e(nom)}</a>')
    elements = [("/", "Accueil"), ("/blog/", "Blog")] + ([(chemin, cat_active)] if cat_active else [])
    fil, fil_ld = fil_ariane(elements)
    objet = {"@context": "https://schema.org", "@type": "Blog" if cat_active is None else "CollectionPage", "name": titre_page,
             "description": chapo, "url": URL + chemin, "inLanguage": "fr-FR",
             "blogPost" if cat_active is None else "hasPart": [{"@type": "BlogPosting", "headline": a["titre"], "url": URL + a["url"],
                                                                 "datePublished": a["date"].isoformat()} for a in articles]}
    une, reste = (articles[0], articles[1:]) if articles and cat_active is None else (None, articles)
    bloc_une = ""
    if une:
        bloc_une = f"""<article class="billet-une apparait">
      <a href="{une["url"]}" class="billet-une-image" tabindex="-1" aria-hidden="true">{img(une["image"] or "/assets/img/linux-travaux.webp")}</a>
      <div>
        <p class="carte-billet-meta"><span class="pastille-cat cat-{CATEGORIES[une["categorie"]]}">{e(une["categorie"])}</span> <time datetime="{une["date"].isoformat()}">{date_fr(une["date"])}</time> · {une["lecture"]} min</p>
        <h2><a href="{une["url"]}">{e(une["titre"])}</a></h2>
        <p>{e(une["description"])}</p>
        <a class="bouton bouton-clair bouton-petit" href="{une["url"]}">Lire l'article</a>
      </div>
    </article>"""
    return tete(f"{titre_page} · E-KO RIP Center", chapo, chemin, extra=ld(objet, fil_ld)) + f"""
<main id="contenu">
<section class="heros heros-court">
  <div class="heros-fond" aria-hidden="true"><span class="c"></span><span class="m"></span><span class="j"></span></div>
  <div class="conteneur">
    {fil}
    <span class="surtitre apparait">{e(surtitre)}</span>
    <h1 class="apparait">{e(titre_page)}</h1>
    <p class="chapo apparait">{e(chapo)}</p>
    <nav class="filtres-blog apparait" aria-label="Catégories">{"".join(filtres)}</nav>
  </div>
</section>
<section class="section-blog">
  <div class="conteneur">
    {bloc_une}
    <div class="grille-billets">
      {(chr(10) + "      ").join(carte(a, "h2") for a in reste)}
    </div>
    <p class="rss-lien"><a href="/blog/rss.xml">Suivre le blog par flux RSS</a></p>
  </div>
</section>
{appel_final()}
</main>
""" + queue()


def rss(articles):
    maintenant = format_datetime(datetime.combine(articles[0]["date"], datetime.min.time(), timezone.utc)) if articles else ""
    items = "".join(f"""  <item>
    <title>{e(a["titre"])}</title>
    <link>{URL}{a["url"]}</link>
    <guid isPermaLink="true">{URL}{a["url"]}</guid>
    <pubDate>{format_datetime(datetime.combine(a["date"], datetime.min.time(), timezone.utc))}</pubDate>
    <category>{e(a["categorie"])}</category>
    <description>{e(a["description"])}</description>
  </item>
""" for a in articles)
    return f"""<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
<channel>
  <title>Blog E-KO RIP Center</title>
  <link>{URL}/blog/</link>
  <atom:link href="{URL}/blog/rss.xml" rel="self" type="application/rss+xml"/>
  <description>Guides d'atelier, couleur, découpe et comparatifs autour du RIP grand format libre E-KO RIP Center.</description>
  <language>fr-FR</language>
  <lastBuildDate>{maintenant}</lastBuildDate>
{items}</channel>
</rss>
"""


# ---------------------------------------------------------------- feuille de route

def page_feuille_de_route(chemin_md):
    meta, corps = lire_markdown(chemin_md, ["titre", "description", "maj"])
    maj = date.fromisoformat(meta["maj"])
    m = re.match(r"(.*?)(?=^## )", corps, flags=re.M | re.S)
    chapo_md = m[1] if m else ""
    sections = re.findall(r"^## (.+?)\s*$(.*?)(?=^## |\Z)", corps, flags=re.M | re.S)
    noms = [s[0].strip() for s in sections]
    if noms != COLONNES_FDR:
        erreur(f"{chemin_md.relative_to(RACINE)} : sections attendues {COLONNES_FDR}, trouvées {noms}")
    vus = {}
    colonnes = []
    classes = {"Fait": "fait", "En cours": "en-cours", "Planifié": "planifie"}
    for nom, bloc in sections:
        nom = nom.strip()
        for item in re.findall(r"^\s*[-*] \*\*(.+?)\*\*", bloc, flags=re.M):
            if item in vus:
                erreur(f"{chemin_md.relative_to(RACINE)} : « {item} » est à la fois dans {vus[item]} et {nom}")
            vus[item] = nom
        contenu, _ = convertir(chemin_md, bloc)
        n = len(re.findall(r"^\s*[-*] ", bloc, flags=re.M))
        colonnes.append(f"""<section class="fdr-colonne fdr-{classes[nom]}" aria-labelledby="fdr-{classes[nom]}">
      <h2 id="fdr-{classes[nom]}"><span class="fdr-puce" aria-hidden="true"></span>{e(nom)} <span class="fdr-nombre">{n}</span></h2>
      <div class="doc-article">{contenu}</div>
    </section>""")
    chapo, _ = convertir(chemin_md, chapo_md)
    fil, fil_ld = fil_ariane([("/", "Accueil"), ("/feuille-de-route/", meta["titre"])])
    return tete(f'{meta["titre"]} · E-KO RIP Center', meta["description"], "/feuille-de-route/", extra=ld(fil_ld), prose=True) + f"""
<main id="contenu">
<section class="heros heros-court">
  <div class="heros-fond" aria-hidden="true"><span class="c"></span><span class="m"></span><span class="j"></span></div>
  <div class="conteneur">
    {fil}
    <span class="surtitre apparait">Mise à jour le <time datetime="{maj.isoformat()}">{date_fr(maj)}</time></span>
    <h1 class="apparait">{e(meta["titre"])}</h1>
    <div class="chapo apparait">{chapo}</div>
  </div>
</section>
<section class="section-fdr">
  <div class="conteneur fdr-grille">
    {(chr(10) + "    ").join(colonnes)}
  </div>
</section>
{appel_final()}
</main>
""" + queue()


# ---------------------------------------------------------------- pages écrites à la main

def pages_manuelles():
    for f in sorted(SITE.rglob("*.html")):
        rel = f.relative_to(SITE).as_posix()
        if rel.startswith(DOSSIERS_GENERES) or f.name not in SOURCE_PAGES:
            continue
        yield rel, f


def adresse(rel):
    if rel == "404.html":
        return "/404"
    return "/" + rel[: -len("index.html")]


def remplacer(texte, balise, contenu, rel):
    motif = re.compile(rf"<!-- {balise} -->.*?<!-- /{balise} -->", flags=re.S)
    if not motif.search(texte):
        erreur(f"site/{rel} : repères <!-- {balise} --> … <!-- /{balise} --> introuvables")
    return motif.sub(lambda _: f"<!-- {balise} -->\n{contenu}\n<!-- /{balise} -->", texte, count=1)


def billets_recents(articles):
    return f"""<div class="grille-billets">
      {(chr(10) + "      ").join(carte(a) for a in articles[:3])}
    </div>"""


# ---------------------------------------------------------------- plan du site et llms.txt

def plan(articles, autres):
    urls = ["/"] + sorted(set(autres) - {"/", "/404"}) + ["/blog/"] + \
        [f"/blog/categorie/{s}/" for n, s in CATEGORIES.items() if any(a["categorie"] == n for a in articles)] + \
        [a["url"] for a in articles]
    dates = {a["url"]: a["maj"].isoformat() for a in articles}
    lignes = "".join(f"  <url><loc>{URL}{u}</loc>" + (f"<lastmod>{dates[u]}</lastmod>" if u in dates else "") + "</url>\n" for u in urls)
    return '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n' + lignes + "</urlset>\n"


def llms(articles):
    par_cat = {}
    for a in articles:
        par_cat.setdefault(a["categorie"], []).append(a)
    blog = "".join(f"\n### {cat}\n\n" + "".join(f"- [{a['titre']}]({URL}{a['url']}) : {a['description']}\n" for a in lst)
                   for cat, lst in par_cat.items())
    return f"""# E-KO RIP Center

> E-KO RIP Center est un RIP d'impression grand format libre et gratuit (licence AGPL-3.0), édité par GOLDEN PRINT (France). Un serveur Linux (Ubuntu ou Debian) rend les PDF en plaques d'encre CMJN, blanc, argent et tons directs, prépare l'impression et la découpe, et pilote les machines de l'atelier. Clients natifs macOS, Linux et Windows, application mobile, et Cloud E-KO RIP pour l'accès à distance sans VPN.

Points clés :

- Prix : logiciel gratuit, sans licence par poste, sans clé matérielle ni serveur d'activation. Le Cloud est gratuit pour un petit atelier, puis facturé à la machine ({URL}/cloud/).
- Code source : https://github.com/mvignaud/eko-rip-center
- Contact : contact@e-ko.rip, formulaire {URL}/contact/
- Langue : site et blog en français, documentation en français et en anglais.

## Pages principales

- [Accueil]({URL}/) : présentation, fonctionnalités, fonctionnement, installation
- [Télécharger]({URL}/telecharger/) : serveur, clients Mac, Linux et Windows, application mobile
- [Machines compatibles]({URL}/compatibilite/) : imprimantes et découpeuses pilotées, demande de pilote
- [Pré-requis]({URL}/prerequis/) : serveur, postes, réseau
- [Cloud et tarifs]({URL}/cloud/)
- [Feuille de route]({URL}/feuille-de-route/) : fait, en cours, planifié
- [Nouveautés]({URL}/changelog/) : journal des versions
- [Documentation (FR)]({URL}/documentation/) et [documentation (EN)]({URL}/documentation/en/)
- [Contact]({URL}/contact/)

## Blog
{blog}"""


# ---------------------------------------------------------------- assemblage

def construire():
    """Rend {chemin relatif à site/ : contenu} pour tout ce que le générateur écrit."""
    articles = lire_articles()
    sortie = {}
    for a in articles:
        sortie[f"blog/{a['slug']}/index.html"] = page_article(a, articles)
    sortie["blog/index.html"] = liste_billets(
        articles, articles, "Le blog de l'atelier", "Guides pratiques, couleur, découpe et comparatifs : ce que nous apprenons en faisant tourner un RIP grand format tous les jours.",
        "/blog/", "Blog")
    for nom, slug in CATEGORIES.items():
        lst = [a for a in articles if a["categorie"] == nom]
        if lst:
            sortie[f"blog/categorie/{slug}/index.html"] = liste_billets(lst, articles, nom, DESCRIPTIONS_CAT[nom], f"/blog/categorie/{slug}/", "Blog", nom)
    sortie["blog/rss.xml"] = rss(articles)
    fdr = PAGES / "feuille-de-route.md"
    if fdr.is_file():
        sortie["feuille-de-route/index.html"] = page_feuille_de_route(fdr)
    autres = []
    for rel, f in pages_manuelles():
        texte = f.read_text(encoding="utf-8")
        texte = remplacer(texte, "entete", site_commun.entete("fr", adresse(rel)), rel)
        texte = remplacer(texte, "pied", site_commun.pied("fr"), rel)
        if "<!-- blog-recents -->" in texte:
            texte = remplacer(texte, "blog-recents", billets_recents(articles), rel)
        sortie[rel] = texte
        autres.append(adresse(rel))
    if fdr.is_file():
        autres.append("/feuille-de-route/")
    sortie["sitemap.xml"] = plan(articles, autres)
    sortie["llms.txt"] = llms(articles)
    verifier_liens(sortie)
    return sortie


def verifier_liens(sortie):
    """Chaque lien interne d'une page générée ou modifiée mène à une page qui existe."""
    existe = set(sortie) | {f.relative_to(SITE).as_posix() for f in SITE.rglob("*") if f.is_file()}
    speciaux = {"/telecharger/mac", "/telecharger/windows"}
    fautes = []
    for rel, texte in sortie.items():
        if not rel.endswith(".html"):
            continue
        for url in re.findall(r'href="(/[^"#?]*)', texte):
            if url in speciaux or url.startswith("//"):
                continue
            cible = url.lstrip("/")
            if not (cible in existe or (cible.rstrip("/") + "/index.html").lstrip("/") in existe or (cible == "" and "index.html" in existe)):
                fautes.append(f"site/{rel} → {url}")
    if fautes:
        erreur("liens internes cassés :\n  " + "\n  ".join(sorted(set(fautes))[:40]))


def generes_sur_disque():
    fichiers = {}
    for dossier in ("blog",):
        d = SITE / dossier
        if d.is_dir():
            fichiers.update({f.relative_to(SITE).as_posix(): f for f in d.rglob("*") if f.is_file()})
    return fichiers


def main():
    sortie = construire()
    if "--verifier" in sys.argv:
        ecarts = [rel for rel, texte in sortie.items()
                  if not (SITE / rel).is_file() or (SITE / rel).read_text(encoding="utf-8") != texte]
        ecarts += [rel for rel in generes_sur_disque() if rel not in sortie]
        if ecarts:
            print("site/ n'est pas à jour avec blog/, pages/ ou le menu : lancer python3 outils/generer-site.py", file=sys.stderr)
            for rel in sorted(ecarts)[:20]:
                print(f"  {rel}", file=sys.stderr)
            sys.exit(1)
        print("Site à jour.")
        return
    for rel in generes_sur_disque():
        if rel not in sortie:
            (SITE / rel).unlink()
    for rel, texte in sortie.items():
        chemin = SITE / rel
        chemin.parent.mkdir(parents=True, exist_ok=True)
        if not chemin.is_file() or chemin.read_text(encoding="utf-8") != texte:
            chemin.write_text(texte, encoding="utf-8")
    n = sum(1 for r in sortie if r.startswith("blog/") and r.endswith("/index.html") and "/categorie/" not in r) - 1
    print(f"Site écrit : {n} articles, {len(sortie)} fichiers.")


if __name__ == "__main__":
    main()
