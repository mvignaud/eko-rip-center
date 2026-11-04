// E-KO RIP Center — chat en direct (support Suite E-KO, Chatwoot).
(function () {
  'use strict';
  var BASE = 'https://help.suite-eko.com';
  var lang = (document.documentElement.lang || 'fr').slice(0, 2);
  window.chatwootSettings = { locale: lang === 'en' ? 'en' : 'fr', position: 'right', type: 'standard', launcherTitle: lang === 'en' ? 'Chat with us' : 'Discuter avec nous' };
  var s = document.createElement('script');
  s.src = BASE + '/packs/js/sdk.js';
  s.defer = true;
  s.async = true;
  s.onload = function () {
    window.chatwootSDK.run({ websiteToken: 'Wokn8ZeMGSBx8n114mqsgize', baseUrl: BASE });
  };
  document.head.appendChild(s);
})();
