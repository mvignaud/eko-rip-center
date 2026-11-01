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

  // Menu mobile.
  var bouton = document.querySelector('.nav-bouton');
  var nav = document.getElementById('nav');
  if (bouton && nav) {
    bouton.addEventListener('click', function () {
      var ouvert = nav.classList.toggle('ouverte');
      bouton.setAttribute('aria-expanded', ouvert ? 'true' : 'false');
    });
    nav.addEventListener('click', function (e) {
      if (e.target.tagName === 'A') { nav.classList.remove('ouverte'); bouton.setAttribute('aria-expanded', 'false'); }
    });
  }

  // Maquettes : macOS, Windows, Linux, clair ou sombre.
  var maquette = document.getElementById('maquette');
  var legende = document.getElementById('legende-os');
  var legendes = {
    mac: 'Client macOS en SwiftUI, natif, universel Apple Silicon et Intel.',
    windows: 'Client Windows en WinUI 3, en préparation : aperçu de l\'interface prévue.',
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

  // Demande de pilote (/compatibilite/) : envoi sans recharger la page ; sans JavaScript,
  // le formulaire part quand même et le script renvoie ici avec ?demande=envoyee|erreur.
  var demande = document.getElementById('demande-pilote');
  if (demande) {
    var retour = demande.querySelector('.message');
    var etatUrl = new URLSearchParams(location.search).get('demande');
    if (etatUrl === 'envoyee') retour.textContent = 'Merci, votre demande est envoyée. Nous vous répondons par e-mail.';
    if (etatUrl === 'erreur') retour.textContent = 'La demande n\'est pas partie. Vérifiez les champs, ou écrivez à contact@e-ko.rip.';
    demande.addEventListener('submit', function (e) {
      if (!window.fetch) return;
      e.preventDefault();
      var bouton = demande.querySelector('button[type="submit"]');
      bouton.disabled = true;
      retour.textContent = 'Envoi…';
      fetch(demande.action, {
        method: 'POST',
        body: new URLSearchParams(new FormData(demande)),
        headers: { Accept: 'application/json' }
      }).then(function (r) {
        return r.json().catch(function () { return {}; });
      }).then(function (j) {
        retour.textContent = j.message || 'La demande n\'est pas partie. Écrivez-nous à contact@e-ko.rip.';
        if (j.ok) demande.reset();
      }).catch(function () {
        retour.textContent = 'La demande n\'est pas partie. Écrivez-nous à contact@e-ko.rip.';
      }).then(function () { bouton.disabled = false; });
    });
  }

  document.querySelectorAll('[data-annee]').forEach(function (n) { n.textContent = String(new Date().getFullYear()); });
})();
