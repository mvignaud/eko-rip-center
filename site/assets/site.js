// E-KO RIP Center — comportements du site : apparitions, maquettes par système, onglets, visionneuse.
(function () {
  'use strict';

  // Apparition au défilement (et déclenchement des animations de la scène, des plaques et du trajet).
  var cibles = document.querySelectorAll('.apparait, .apparait-seul, .scene, .pile');
  if ('IntersectionObserver' in window) {
    var obs = new IntersectionObserver(function (entrees) {
      entrees.forEach(function (e) {
        if (e.isIntersecting) { e.target.classList.add('visible'); obs.unobserve(e.target); }
      });
    }, { threshold: 0.15, rootMargin: '0px 0px -40px 0px' });
    cibles.forEach(function (c) { obs.observe(c); });
  } else {
    cibles.forEach(function (c) { c.classList.add('visible'); });
  }

  // Menu : bouton du téléphone, et rubriques déroulantes (clic, clavier, Échap, clic ailleurs).
  var bouton = document.querySelector('.nav-bouton');
  var nav = document.getElementById('nav');
  var titres = document.querySelectorAll('.nav-groupe > button.nav-titre');
  function fermerRubriques(sauf) {
    titres.forEach(function (t) { if (t !== sauf) t.setAttribute('aria-expanded', 'false'); });
  }
  titres.forEach(function (t) {
    t.addEventListener('click', function (e) {
      e.stopPropagation();
      var ouvert = t.getAttribute('aria-expanded') === 'true';
      fermerRubriques(t);
      t.setAttribute('aria-expanded', ouvert ? 'false' : 'true');
    });
  });
  document.addEventListener('click', function (e) {
    if (!e.target.closest || !e.target.closest('.nav-groupe')) fermerRubriques(null);
  });
  document.addEventListener('keydown', function (e) {
    if (e.key !== 'Escape') return;
    var ouvert = document.querySelector('.nav-groupe > button[aria-expanded="true"]');
    fermerRubriques(null);
    if (ouvert) ouvert.focus();
  });
  if (bouton && nav) {
    bouton.addEventListener('click', function () {
      var ouvert = nav.classList.toggle('ouverte');
      bouton.setAttribute('aria-expanded', ouvert ? 'true' : 'false');
    });
    nav.addEventListener('click', function (e) {
      if (e.target.closest && e.target.closest('a')) {
        nav.classList.remove('ouverte');
        bouton.setAttribute('aria-expanded', 'false');
        fermerRubriques(null);
      }
    });
  }

  // Maquettes : macOS, Windows, Linux, clair ou sombre.
  var maquette = document.getElementById('maquette');
  var legende = document.getElementById('legende-os');
  var legendes = {
    mac: 'Client macOS en SwiftUI, natif, universel Apple Silicon et Intel.',
    windows: 'Client Windows natif en WinUI 3, pour Windows 10 et 11.',
    linux: 'Client Linux en GTK 4 et libadwaita, installé et mis à jour par APT.'
  };
  var systeme = 'mac';
  var auto = true;
  function choisir(os) {
    if (!maquette) return;
    systeme = os;
    maquette.classList.remove('mac', 'windows', 'linux');
    maquette.classList.add(os);
    document.querySelectorAll('.systemes [data-os]').forEach(function (b) {
      b.setAttribute('aria-selected', b.getAttribute('data-os') === os ? 'true' : 'false');
    });
    if (legende) legende.textContent = legendes[os];
  }
  document.querySelectorAll('.systemes [data-os]').forEach(function (b) {
    b.addEventListener('click', function () { auto = false; choisir(b.getAttribute('data-os')); });
  });
  var bascule = document.querySelector('[data-theme-bascule]');
  if (maquette && window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
    maquette.classList.add('sombre');
  }
  function majBascule() {
    if (bascule && maquette) bascule.textContent = maquette.classList.contains('sombre') ? 'Clair' : 'Sombre';
  }
  majBascule();
  if (bascule) {
    bascule.addEventListener('click', function () { auto = false; maquette.classList.toggle('sombre'); majBascule(); });
  }
  // Tour automatique des trois systèmes tant que le visiteur n'a rien choisi.
  var calme = window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (maquette && !calme) {
    var ordre = ['mac', 'windows', 'linux'];
    setInterval(function () {
      if (!auto || document.hidden) return;
      choisir(ordre[(ordre.indexOf(systeme) + 1) % ordre.length]);
    }, 4500);
  }

  // Onglets d'installation.
  var onglets = document.querySelectorAll('.onglets-install [role="tab"]');
  onglets.forEach(function (o) {
    o.addEventListener('click', function () {
      onglets.forEach(function (x) {
        var actif = x === o;
        x.setAttribute('aria-selected', actif ? 'true' : 'false');
        var p = document.getElementById(x.getAttribute('aria-controls'));
        if (p) p.hidden = !actif;
      });
    });
  });

  // Visionneuse des captures.
  document.querySelectorAll('.capture img').forEach(function (img) {
    img.addEventListener('click', function () {
      var v = document.createElement('div');
      v.className = 'visionneuse';
      v.setAttribute('role', 'dialog');
      v.setAttribute('aria-label', img.alt);
      var grande = document.createElement('img');
      grande.src = img.currentSrc || img.src;
      grande.alt = img.alt;
      v.appendChild(grande);
      function fermer() { v.remove(); document.removeEventListener('keydown', touche); }
      function touche(e) { if (e.key === 'Escape') fermer(); }
      v.addEventListener('click', fermer);
      document.addEventListener('keydown', touche);
      document.body.appendChild(v);
    });
  });

  // Formulaires (demande de pilote, contact) : envoi sans recharger la page ; sans JavaScript,
  // le formulaire part quand même et le script renvoie sur l'ancre du message (affiché par :target).
  document.querySelectorAll('#demande-pilote, .formulaire-envoi').forEach(function (formulaire) {
    var retour = formulaire.querySelector('.message');
    var repli = 'Le message n\'est pas parti. Écrivez-nous à contact@e-ko.rip.';
    formulaire.addEventListener('submit', function (e) {
      if (!window.fetch) return;
      e.preventDefault();
      var envoi = formulaire.querySelector('button[type="submit"]');
      envoi.disabled = true;
      retour.textContent = 'Envoi…';
      fetch(formulaire.action, {
        method: 'POST',
        body: new URLSearchParams(new FormData(formulaire)),
        headers: { Accept: 'application/json' }
      }).then(function (r) {
        return r.json().catch(function () { return {}; });
      }).then(function (j) {
        retour.textContent = j.message || repli;
        if (j.ok) formulaire.reset();
      }).catch(function () {
        retour.textContent = repli;
      }).then(function () { envoi.disabled = false; });
    });
  });

  // Page Télécharger : version, taille et empreinte SHA-256 des fichiers de la dernière release.
  var fiches = document.querySelectorAll('[data-fichier]');
  if (fiches.length && window.fetch) {
    fetch('/telecharger/infos.json', { headers: { Accept: 'application/json' } }).then(function (r) {
      return r.ok ? r.json() : {};
    }).then(function (infos) {
      fiches.forEach(function (fiche) {
        var i = infos[fiche.getAttribute('data-fichier')];
        if (!i) return;
        function champ(nom, valeur) {
          var n = fiche.querySelector('[data-champ="' + nom + '"]');
          if (n && valeur) n.textContent = valeur;
        }
        champ('version', i.version + (i.date ? ' du ' + i.date.split('-').reverse().join('/') : ''));
        champ('fichier', i.fichier);
        champ('taille', i.taille ? (i.taille / 1048576).toLocaleString('fr-FR', { maximumFractionDigits: 1 }) + ' Mo' : '');
        if (i.sha256) {
          var n = fiche.querySelector('[data-champ="sha256"]');
          n.textContent = '';
          var c = document.createElement('code');
          c.textContent = i.sha256;
          n.appendChild(c);
        }
      });
    }).catch(function () {});
  }

  document.querySelectorAll('[data-annee]').forEach(function (n) { n.textContent = String(new Date().getFullYear()); });
})();
