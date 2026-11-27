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
    "info.rules": "Rules",
    "info.pending": "Still to be announced:",
    "info.dateValue": "November 28–29",
    "live.title": "Watch live",
    "players.title": "Player ranking",
    "players.note": "Tied on points? More MVPs ranks higher, then fewer matches played, then more wins.",
    "players.rank": "#",
    "players.player": "Player",
    "players.score": "Points",
    "players.mvp": "MVPs",
    "players.wins": "Wins",
    "players.played": "Matches",
    "players.rate": "Win rate",
    "players.empty": "Ranking coming soon",
    "players.more": "See more",
    "players.less": "See less",
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
    "hist.day1": "Day 1",
    "hist.day2": "Day 2",
    "hist.final": "Final",
    "match.label": "Match",
    "match.duration": "Duration",
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
    "live.on": "Live",
    "live.offline": "is offline right now.",
    "live.offlineHint": "When the stream starts, it shows up here on its own.",
    "live.watch": "Watch",
    "live.openShort": "Open on Twitch",
    "live.openPre": "Open",
    "live.openPost": "on Twitch",
    "live.gate": "Asked you to log in? Some channels are age-restricted and only open on Twitch itself.",
    "live.open": "Open on Twitch",
    "live.local": "The stream only plays on the published site (Twitch blocks it when the file is opened directly).",
    "hero.sub": "Community-run Deadlock tournament",
    "rules.title": "Format and prizes",
    "rules.f1": "Schedule",
    "rules.f1.1": "Saturday, Nov 28: starts at 1 PM, with at least 10 matches planned. The day is expected to end at 6:30 PM.",
    "rules.f1.2": "Sunday, Nov 29: starts at 2 PM and runs until the tournament ends. 11 matches are planned, including the three finals, played by the 12 best-ranked players.",
    "rules.f2": "How the MIX works",
    "rules.f2.1": "Teams are drawn at random. To balance them, each team has one player from the “Captain” category. This label is only used to organize the draw.",
    "rules.f2.2": "If players think the split is unbalanced or want to play with a different lineup, they can ask for a refresh to redraw the team.",
    "rules.f3": "Standings",
    "rules.f3.1": "Standings are individual. Before the finals, each win is worth 2 points and each loss is worth 1 point.",
    "rules.f3.2": "In the three final matches, each win is worth 3 points, and each loss is still worth 1 point.",
    "rules.f4": "Live broadcast",
    "rules.f4.1": "Two matches run at the same time: Farinha streams one and Nokeime streams the other.",
    "rules.prizeText": "The total prize, currently set at R$ 1,000.00, will be split among the 6 best-ranked players at the end of the tournament.",
    "collab.title": "Contributors",
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
    "info.rules": "Reglas",
    "info.pending": "Aún por definir:",
    "info.dateValue": "28 y 29 de noviembre",
    "live.title": "Mira en vivo",
    "players.title": "Ranking de jugadores",
    "players.note": "¿Empate en puntos? Queda delante quien tenga más MVP; después, quien jugó menos partidas; al final, quien ganó más.",
    "players.rank": "#",
    "players.player": "Jugador",
    "players.score": "Puntos",
    "players.mvp": "MVP",
    "players.wins": "Victorias",
    "players.played": "Partidas",
    "players.rate": "Efectividad",
    "players.empty": "Ranking próximamente",
    "players.more": "Ver más",
    "players.less": "Ver menos",
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
    "hist.day1": "Día 1",
    "hist.day2": "Día 2",
    "hist.final": "Final",
    "match.label": "Partida",
    "match.duration": "Duración",
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
    "live.on": "En vivo",
    "live.offline": "está desconectado ahora.",
    "live.offlineHint": "Cuando empiece la transmisión, aparecerá aquí sola.",
    "live.watch": "Ver",
    "live.openShort": "Abrir en Twitch",
    "live.openPre": "Abrir",
    "live.openPost": "en Twitch",
    "live.gate": "¿Te pidió iniciar sesión? Algunos canales tienen restricción de edad y solo abren en Twitch.",
    "live.open": "Abrir en Twitch",
    "live.local": "La transmisión solo aparece en el sitio publicado (Twitch la bloquea al abrir el archivo directamente).",
    "hero.sub": "Torneo de Deadlock hecho por la comunidad",
    "rules.title": "Formato y premios",
    "rules.f1": "Programación",
    "rules.f1.1": "Sábado 28/11: empieza a las 13 h, con al menos 10 partidas previstas. El cierre del día está previsto para las 18:30.",
    "rules.f1.2": "Domingo 29/11: empieza a las 14 h y sigue hasta el final del campeonato. Se prevén 11 partidas, incluidas las tres finales, disputadas por los 12 jugadores mejor clasificados.",
    "rules.f2": "Cómo funciona el MIX",
    "rules.f2.1": "Los equipos se forman por sorteo. Para equilibrarlos, cada uno tiene un jugador de la categoría “Capitán”. Esta identificación solo sirve para organizar el sorteo.",
    "rules.f2.2": "Si creen que la división quedó desequilibrada o quieren jugar con otra formación, los jugadores pueden pedir un refresh para sortear el equipo de nuevo.",
    "rules.f3": "Clasificación",
    "rules.f3.1": "La clasificación es individual. Antes de las finales, cada victoria vale 2 puntos y cada derrota vale 1 punto.",
    "rules.f3.2": "En las tres partidas finales, cada victoria pasa a valer 3 puntos y cada derrota sigue valiendo 1 punto.",
    "rules.f4": "Transmisión en vivo",
    "rules.f4.1": "Se juegan dos partidas al mismo tiempo: Farinha transmite una y Nokeime transmite la otra.",
    "rules.prizeText": "El premio total, por ahora fijado en R$ 1.000,00, se dividirá entre los 6 jugadores mejor clasificados al final del campeonato.",
    "collab.title": "Colaboradores",
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
  pt: "Refresher League é um campeonato de Deadlock feito pela comunidade. Acompanhe as lives, o ranking e as últimas partidas.",
  en: "Refresher League is a community-run Deadlock tournament. Follow the streams, the ranking and the latest matches.",
  es: "Refresher League es un torneo de Deadlock hecho por la comunidad. Sigue las transmisiones, el ranking y las últimas partidas."
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
  // Números e datas no formato do idioma escolhido (1.234 / 1,234; 17/11/2026 / 11/17/2026)
  const loc = htmlLang[current] || "pt-BR";
  root.querySelectorAll("[data-num]").forEach((el) => { el.textContent = Number(el.dataset.num).toLocaleString(loc); });
  root.querySelectorAll("[data-date]").forEach((el) => { el.textContent = new Date(+el.dataset.date).toLocaleDateString(loc); });
  root.querySelectorAll("[data-datetime]").forEach((el) => {
    const d = new Date(+el.dataset.datetime);
    el.textContent = d.toLocaleDateString(loc) + " · " + d.toLocaleTimeString(loc, { hour: "2-digit", minute: "2-digit" });
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
