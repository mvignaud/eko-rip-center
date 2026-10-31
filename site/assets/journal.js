// E-KO RIP Center — le journal des versions.
//
// La source reste le journal de l'ERP (resources/changelog-rip/*.yaml d'eko-source), qui envoie
// aussi l'e-mail aux abonnés à chaque déploiement : le site ne fait que l'afficher.
// 1. On lit le journal en direct sur l'ERP (toujours à jour).
// 2. Si l'ERP ne répond pas, on lit la copie /changelog.<langue>.json déposée avec le site.
(function () {
  'use strict';

  var ERP = 'https://app.e-ko.fr';
  var LANGUES = ['fr', 'en'];
  var params = new URLSearchParams(location.search);
  var langue = LANGUES.indexOf(params.get('lang')) >= 0 ? params.get('lang') : 'fr';

  var TEXTES = {
    fr: {
      categories: { features: 'Nouveauté', fix: 'Correction', fixes: 'Correction', bugfix: 'Correction', security: 'Sécurité', optimisation: 'Optimisation', enhancements: 'Amélioration', ux: 'Ergonomie', docs: 'Documentation', i18n: 'Traduction', kernel: 'Technique', tests: 'Tests', modules: 'Module', greenness: 'Sobriété' },
      erreur: 'Le journal est momentanément indisponible.',
      vide: 'Aucune version publiée pour l\'instant.',
      envoi: 'Envoi…',
      erreurAbo: 'L\'inscription n\'a pas abouti, réessayez dans un instant.'
    },
    en: {
      categories: { features: 'New', fix: 'Fix', fixes: 'Fix', bugfix: 'Fix', security: 'Security', optimisation: 'Performance', enhancements: 'Improvement', ux: 'Usability', docs: 'Docs', i18n: 'Translation', kernel: 'Technical', tests: 'Tests', modules: 'Module', greenness: 'Efficiency' },
      erreur: 'The changelog is temporarily unavailable.',
      vide: 'No release published yet.',
      envoi: 'Sending…',
      erreurAbo: 'Subscription failed, please try again in a moment.'
    }
  };
  var T = TEXTES[langue];

  function echapper(s) {
    return String(s == null ? '' : s).replace(/[&<>"']/g, function (c) {
      return { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', '\'': '&#39;' }[c];
    });
  }
  // Le journal n'utilise que le gras en Markdown.
  function enrichir(s) { return echapper(s).replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>'); }

  function date(iso) {
    var d = new Date(iso + 'T12:00:00');
    if (isNaN(d)) return echapper(iso);
    return d.toLocaleDateString(langue === 'en' ? 'en-GB' : 'fr-FR', { day: 'numeric', month: 'long', year: 'numeric' });
  }

  // Au-delà de 8 s sans réponse, on passe à la copie de secours (revue Graphify).
  function lire(url) {
    var options = { headers: { Accept: 'application/json' } };
    if (window.AbortController) {
      var arret = new AbortController();
      setTimeout(function () { arret.abort(); }, 8000);
      options.signal = arret.signal;
    }
    return fetch(url, options).then(function (r) {
      if (!r.ok) throw new Error('HTTP ' + r.status);
      return r.json();
    }).then(function (j) {
      if (!j || !Array.isArray(j.releases)) throw new Error('journal illisible');
      return j.releases;
    });
  }

  function charger() {
    return lire(ERP + '/changelog-rip.json?lang=' + langue).catch(function () {
      // Copie déposée par outils/publier-site.sh, une par langue.
      return lire('/changelog.' + langue + '.json');
    });
  }

  function versionHtml(r, complet) {
    var entrees = (r.entries || []).map(function (e) {
      var brute = e.category || 'features';
      return '<div class="entree"><span class="cat ' + echapper(brute) + '">' + echapper(T.categories[brute] || brute) + '</span>' +
        '<h3>' + enrichir(e.title) + '</h3>' +
        (complet && e.description ? '<p>' + enrichir(e.description) + '</p>' : '') + '</div>';
    }).join('');
    var ancre = 'v' + String(r.version).replace(/[^0-9a-z.-]/gi, '');
    // Le lien permanent garde la langue lue (revue Codex).
    var page = '/changelog/' + (langue === 'fr' ? '' : '?lang=' + langue);
    return '<article class="version" id="' + ancre + '">' +
      '<div class="version-tete"><h2><a href="' + page + '#' + ancre + '" style="color:inherit">' + echapper(r.version) + '</a></h2><time datetime="' + echapper(r.date) + '">' + date(r.date) + '</time></div>' +
      (r.codename ? '<p class="devise">' + enrichir(r.codename) + '</p>' : '') +
      entrees + '</article>';
  }

  var complet = document.getElementById('journal');
  var apercu = document.getElementById('apercu-journal');
  var etiquettes = document.querySelectorAll('[data-derniere-version]');

  if (complet || apercu || etiquettes.length) {
    charger().then(function (releases) {
      if (!releases.length) {
        if (complet) complet.innerHTML = '<p class="chargement">' + T.vide + '</p>';
        return;
      }
      etiquettes.forEach(function (n) { n.textContent = releases[0].version; });
      if (complet) {
        complet.innerHTML = releases.map(function (r) { return versionHtml(r, true); }).join('');
        if (location.hash) {
          var cible = document.getElementById(location.hash.slice(1));
          if (cible) cible.scrollIntoView();
        }
      }
      if (apercu) {
        apercu.innerHTML = releases.slice(0, 2).map(function (r) {
          return versionHtml({ version: r.version, date: r.date, codename: r.codename, entries: (r.entries || []).slice(0, 3) }, false);
        }).join('');
      }
    }).catch(function () {
      if (complet) complet.innerHTML = '<p class="chargement">' + T.erreur + '</p>';
    });
  }

  // Abonnement : l'ERP gère la confirmation par e-mail, le désabonnement et l'envoi à chaque version.
  var form = document.getElementById('abonnement');
  if (form) {
    var message = form.parentNode.querySelector('.message');
    form.addEventListener('submit', function (e) {
      e.preventDefault();
      var donnees = new URLSearchParams(new FormData(form));
      message.textContent = T.envoi;
      fetch(ERP + '/changelog-rip/subscribe', {
        method: 'POST',
        body: donnees,
        headers: { Accept: 'application/json', 'Accept-Language': langue },
        credentials: 'omit'
      }).then(function (r) {
        return r.json().catch(function () { return {}; }).then(function (j) {
          if (!r.ok && !j.message) throw new Error();
          message.textContent = j.message || T.erreurAbo;
          if (j.ok) form.reset();
        });
      }).catch(function () { message.textContent = T.erreurAbo; });
    });
  }
})();
