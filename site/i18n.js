// Traduções do site. O português já está no HTML; aqui ficam inglês e espanhol.
const translations = {
  en: {
    "nav.lang": "Language",
    "patrons.archAlt": "Animated 3D model of the Arch Mother",
    "patrons.kingAlt": "Animated 3D model of the Hidden King",
    "info.date": "Date",
    "info.format": "Format",
    "info.prize": "Prize",
    "info.teams": "Teams",
    "info.tbd": "TBD",
    "info.dateValue": "November 28–29",
    "rules.soon": "Coming soon",
    "live.title": "Watch live",
    "players.title": "Player Ranking",
    "players.rank": "#",
    "players.player": "Player",
    "players.score": "Points",
    "players.empty": "Ranking coming soon",
    "match.title": "Latest matches",
    "match.history": "Match history",
    "match.kills": "kills",
    "match.soulsLow": "souls",
    "match.full": "See full scoreboard",
    "hist.title": "Match history",
    "hist.back": "Back to home",
    "hist.intro": "Click a match to see the full scoreboard.",
    "hist.soon": "Coming soon",
    "hist.tbd": "TBD",
    "hist.won": "won",
    "hist.close": "Close",
    "match.label": "Match",
    "match.win": "Victory",
    "match.loss": "Defeat",
    "match.player": "Player",
    "match.souls": "Souls",
    "match.k": "K",
    "match.d": "D",
    "match.a": "A",
    "match.dmg": "Damage",
    "match.obj": "Obj.",
    "match.heal": "Healing",
    "match.items": "Items",
    "live.open": "Open on Twitch",
    "live.local": "The stream only plays on the published site (Twitch blocks it when the file is opened directly).",
    "cta.title": "Ready to compete?",
    "cta.text": "Sign-ups open soon. Keep an eye on our Discord.",
    "cta.button": "Join the Discord",
    "footer.main": "Refresher League",
    "footer.small": "Independent event, not affiliated with Valve."
  },
  es: {
    "nav.lang": "Idioma",
    "patrons.archAlt": "Modelo 3D animado de la Arch Mother",
    "patrons.kingAlt": "Modelo 3D animado del Hidden King",
    "info.date": "Fecha",
    "info.format": "Formato",
    "info.prize": "Premios",
    "info.teams": "Equipos",
    "info.tbd": "Por definir",
    "info.dateValue": "28 y 29 de noviembre",
    "rules.soon": "Próximamente",
    "live.title": "Mira en vivo",
    "players.title": "Ranking de jugadores",
    "players.rank": "#",
    "players.player": "Jugador",
    "players.score": "Puntos",
    "players.empty": "Ranking próximamente",
    "match.title": "Últimas partidas",
    "match.history": "Historial de partidas",
    "match.kills": "bajas",
    "match.soulsLow": "almas",
    "match.full": "Ver marcador completo",
    "hist.title": "Historial de partidas",
    "hist.back": "Volver al inicio",
    "hist.intro": "Haz clic en una partida para ver el marcador completo.",
    "hist.soon": "Próximamente",
    "hist.tbd": "Por definir",
    "hist.won": "ganó",
    "hist.close": "Cerrar",
    "match.label": "Partida",
    "match.win": "Victoria",
    "match.loss": "Derrota",
    "match.player": "Jugador",
    "match.souls": "Almas",
    "match.k": "K",
    "match.d": "D",
    "match.a": "A",
    "match.dmg": "Daño",
    "match.obj": "Obj.",
    "match.heal": "Curación",
    "match.items": "Objetos",
    "live.open": "Abrir en Twitch",
    "live.local": "La transmisión solo aparece en el sitio publicado (Twitch la bloquea al abrir el archivo directamente).",
    "cta.title": "¿Listo para competir?",
    "cta.text": "Las inscripciones abren pronto. Atento a nuestro Discord.",
    "cta.button": "Unirse al Discord",
    "footer.main": "Refresher League",
    "footer.small": "Evento independiente, sin vínculo con Valve."
  }
};

// Textos dos cartões de informação editados pelo painel da staff (info.js)
if (window.INFO) Object.entries(window.INFO).forEach(([k, v]) => {
  ["en", "es"].forEach((l) => { if (v && v[l]) translations[l]["infoval." + k] = v[l]; });
});

const htmlLang = { pt: "pt-BR", en: "en", es: "es" };
const descriptions = {
  pt: "Refresher League é um campeonato de Deadlock feito pela comunidade. Veja as regras, o formato e como se inscrever.",
  en: "Refresher League is a community-run Deadlock tournament. See the rules, the format and how to sign up.",
  es: "Refresher League es un torneo de Deadlock hecho por la comunidad. Mira las reglas, el formato y cómo inscribirte."
};
const titles = {
  pt: "Refresher League",
  en: "Refresher League",
  es: "Refresher League"
};

// Guarda o texto original em português antes de qualquer troca.
// Conteúdo criado depois (como a janela de uma partida) é traduzido com window.i18nApply(elemento).
translations.pt = {};
let current = "pt";

function apply(root, dict) {
  root.querySelectorAll("[data-i18n]").forEach((el) => {
    const k = el.dataset.i18n;
    if (!(k in translations.pt)) translations.pt[k] = el.textContent;
    const text = dict[k];
    if (text) el.textContent = text;
  });
  root.querySelectorAll("[data-i18n-label]").forEach((el) => {
    const k = el.dataset.i18nLabel;
    if (!(k in translations.pt)) translations.pt[k] = el.getAttribute("aria-label");
    const label = dict[k];
    if (label) el.setAttribute("aria-label", label);
  });
  root.querySelectorAll("[data-i18n-alt]").forEach((el) => {
    const k = el.dataset.i18nAlt;
    if (!(k in translations.pt)) translations.pt[k] = el.getAttribute("alt");
    const alt = dict[k];
    if (alt) el.setAttribute("alt", alt); // atributo: funciona em <img> e em <model-viewer>
  });
}
window.i18nApply = (root) => apply(root, translations[current] || translations.pt);

// Páginas internas (ex.: histórico) dizem o próprio título em <body data-i18n-title="...">
const pageTitle = document.body.dataset.i18nTitle;
const pageTitlePt = document.body.dataset.titlePt;

function setLanguage(lang) {
  current = translations[lang] ? lang : "pt";
  const dict = translations[current];
  apply(document, dict);
  document.documentElement.lang = htmlLang[lang] || "pt-BR";
  if (pageTitle) {
    document.title = (current === "pt" ? pageTitlePt : dict[pageTitle] || pageTitlePt) + " · Refresher League";
  } else {
    document.title = titles[lang] || titles.pt;
    const meta = document.querySelector('meta[name="description"]');
    if (meta) meta.setAttribute("content", descriptions[lang] || descriptions.pt);
  }
  document.querySelectorAll("[data-lang]").forEach((btn) => {
    btn.setAttribute("aria-pressed", String(btn.dataset.lang === lang));
  });
  try { localStorage.setItem("lang", lang); } catch (e) {}
}

document.querySelectorAll("[data-lang]").forEach((btn) => {
  btn.addEventListener("click", () => setLanguage(btn.dataset.lang));
});

// Usa o idioma salvo; se não houver, tenta o idioma do navegador.
let start = "pt";
try { start = localStorage.getItem("lang") || ""; } catch (e) {}
if (!start) {
  const nav = (navigator.language || "pt").slice(0, 2);
  start = translations[nav] ? nav : "pt";
}
setLanguage(start);
