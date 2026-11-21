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
    "players.note": "Tied on points? The player with more MVPs ranks higher.",
    "players.rank": "#",
    "players.player": "Player",
    "players.score": "Points",
    "players.mvp": "MVP",
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
    "rules.title": "Rules, format and prizes",
    "rules.s1": "1. Attendance and schedule",
    "rules.s1.1": "Each player must confirm attendance in the official channel 15 minutes before the scheduled time.",
    "rules.s1.2": "There is a 10-minute grace period after the scheduled time. After that, the referees record the absence and apply the rule set out in the announced format.",
    "rules.s1.3": "Each participant must play on their own Steam account and report any problem directly to the referees.",
    "rules.s2": "2. Matches and technical issues",
    "rules.s2.1": "Matches are played in a private lobby. The organizers will announce the server region and the side and hero selection rules before the tournament.",
    "rules.s2.2": "Pauses are allowed for technical issues. The affected player must notify the referees, who decide when to resume the match, within the game's limits.",
    "rules.s2.3": "If disconnected, a player may try to rejoin during the pause. If the failure affects several participants or prevents the match from continuing, the referees may restart or reschedule it.",
    "rules.s3": "3. Conduct",
    "rules.s3.1": "Cheats, automating actions with scripts or macros, deliberate bug abuse and match fixing are prohibited.",
    "rules.s3.2": "Players may not watch the broadcast of their own match or receive information from spectators while it is in progress, including during pauses.",
    "rules.s3.3": "Harassment, discrimination, threats and insults are subject to penalties.",
    "rules.s3.4": "Only authorized spectators may join the lobby. Authorized broadcasts must have a delay of at least 3 minutes.",
    "rules.s4": "4. Results, disputes and penalties",
    "rules.s4.1": "The organizers record the result of each match and update the individual standings according to the announced scoring system.",
    "rules.s4.2": "Any player may dispute a result or report a violation directly to the referees within 15 minutes after the match ends. The report must include the match ID and evidence, when available. For suspected cheating, the deadline starts when the evidence is found.",
    "rules.s4.3": "The referees may issue a warning, a point deduction or disqualification, depending on severity and repeat offenses. The decision and its reason will be communicated to the affected player.",
    "rules.formatText": "TBD",
    "rules.prizeText": "TBD",
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
    "players.note": "¿Empate en puntos? Queda delante quien tenga más MVP.",
    "players.rank": "#",
    "players.player": "Jugador",
    "players.score": "Puntos",
    "players.mvp": "MVP",
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
    "rules.title": "Reglas, formato y premios",
    "rules.s1": "1. Asistencia y horarios",
    "rules.s1.1": "Cada jugador debe confirmar su asistencia en el canal oficial 15 minutos antes de la hora programada.",
    "rules.s1.2": "La tolerancia es de 10 minutos después de la hora programada. Pasado ese plazo, el arbitraje registra la ausencia y aplica la regla prevista en el formato publicado.",
    "rules.s1.3": "Cada participante debe jugar con su propia cuenta de Steam y comunicar cualquier problema directamente al arbitraje.",
    "rules.s2": "2. Partidas y problemas técnicos",
    "rules.s2.1": "Las partidas se jugarán en lobby privado. La organización anunciará la región del servidor y las reglas de elección de lado y de héroes antes del torneo.",
    "rules.s2.2": "Se permiten pausas en caso de problemas técnicos. El jugador afectado debe avisar al arbitraje, que decide cuándo reanudar la partida, respetando los límites del juego.",
    "rules.s2.3": "En caso de desconexión, el jugador puede intentar volver durante la pausa. Si la falla afecta a varios participantes o impide continuar, el arbitraje puede reiniciar o reprogramar la partida.",
    "rules.s3": "3. Conducta",
    "rules.s3.1": "Están prohibidos los cheats, la automatización de acciones con scripts o macros, el abuso deliberado de bugs y el arreglo de resultados.",
    "rules.s3.2": "Los jugadores no pueden ver la transmisión de su propia partida ni recibir información de espectadores mientras esté en curso, incluso durante las pausas.",
    "rules.s3.3": "El acoso, la discriminación, las amenazas y los insultos están sujetos a sanción.",
    "rules.s3.4": "Solo los espectadores autorizados pueden entrar al lobby. Las transmisiones autorizadas deben tener al menos 3 minutos de retraso.",
    "rules.s4": "4. Resultados, reclamos y sanciones",
    "rules.s4.1": "La organización registra los resultados de cada partida y actualiza la clasificación individual según el sistema de puntuación publicado.",
    "rules.s4.2": "Cualquier jugador puede impugnar un resultado o reportar una infracción directamente al arbitraje hasta 15 minutos después del final de la partida. El reporte debe incluir el ID de la partida y las pruebas, cuando estén disponibles. En caso de sospecha de trampa, el plazo empieza cuando se encuentra la prueba.",
    "rules.s4.3": "El arbitraje puede aplicar una advertencia, pérdida de puntos o descalificación, según la gravedad y la reincidencia. La decisión y el motivo se comunicarán al jugador afectado.",
    "rules.formatText": "Por definir",
    "rules.prizeText": "Por definir",
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
