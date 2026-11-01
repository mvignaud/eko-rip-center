// E-KO RIP Center — documentation : recherche dans le sommaire, section courante dans « Sur cette page ».
(function () {
  'use strict';
  document.documentElement.classList.add('js-doc');
  var menu = document.querySelector('.doc-menu');
  var boutonMenu = document.querySelector('[data-doc-menu]');
  if (menu && boutonMenu) {
    boutonMenu.addEventListener('click', function () {
      var ouvert = menu.classList.toggle('ouvert');
      boutonMenu.setAttribute('aria-expanded', ouvert ? 'true' : 'false');
    });
  }
  var champ = document.querySelector('[data-doc-filtre]');
  var nav = document.querySelector('[data-doc-nav]');
  var liste = document.querySelector('[data-doc-resultats]');
  var index = null;
  var base = document.body.getAttribute('data-doc-langue') === 'en' ? '/documentation/en/' : '/documentation/';

  function sansAccents(t) { return t.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, ''); }

  function afficher(q) {
    var mots = sansAccents(q).split(/\s+/).filter(Boolean);
    liste.textContent = '';
    if (!mots.length) { liste.hidden = true; nav.hidden = false; return; }
    var trouves = index.map(function (e) {
      var titre = sansAccents(e.p + ' ' + e.t), texte = sansAccents(e.x), score = 0;
      for (var i = 0; i < mots.length; i++) {
        if (titre.indexOf(mots[i]) >= 0) score += 3;
        else if (texte.indexOf(mots[i]) >= 0) score += 1;
        else return null;
      }
      return { e: e, score: score };
    }).filter(Boolean).sort(function (a, b) { return b.score - a.score; }).slice(0, 12);
    if (!trouves.length) {
      var vide = document.createElement('li');
      vide.className = 'vide';
      vide.textContent = champ.getAttribute('data-doc-aucun');
      liste.appendChild(vide);
    }
    trouves.forEach(function (r) {
      var li = document.createElement('li'), a = document.createElement('a'), petit = document.createElement('small');
      a.href = r.e.u;
      a.textContent = r.e.t;
      petit.textContent = r.e.p;
      a.appendChild(petit);
      li.appendChild(a);
      liste.appendChild(li);
    });
    liste.hidden = false;
    nav.hidden = true;
  }

  if (champ && nav && liste) {
    champ.addEventListener('input', function () {
      var q = champ.value;
      if (index) { afficher(q); return; }
      fetch(base + 'recherche.json').then(function (r) { return r.json(); }).then(function (d) {
        index = d;
        afficher(champ.value);
      }).catch(function () { index = null; });
    });
    champ.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') { champ.value = ''; afficher(''); }
    });
  }

  // « Sur cette page » : souligne la section lue.
  var liens = document.querySelectorAll('.doc-toc a');
  if (liens.length && 'IntersectionObserver' in window) {
    var parId = {};
    liens.forEach(function (a) { parId[a.getAttribute('href').slice(1)] = a; });
    var obs = new IntersectionObserver(function (entrees) {
      entrees.forEach(function (e) {
        if (!e.isIntersecting) return;
        liens.forEach(function (a) { a.classList.remove('actif'); });
        var a = parId[e.target.id];
        if (a) a.classList.add('actif');
      });
    }, { rootMargin: '-80px 0px -70% 0px' });
    document.querySelectorAll('.doc-article h2[id], .doc-article h3[id]').forEach(function (h) { obs.observe(h); });
  }
}());
