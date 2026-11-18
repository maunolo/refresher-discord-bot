// Monta o placar de uma partida (moldura dourada, os dois times, itens e heróis).
// Usado pela página inicial (últimas partidas) e pelo histórico (janela da partida).
// Precisa de partida.js carregado antes (window.PARTIDAS).
window.renderPartida = (P, numero, titleTag = "h3") => {
  const D = window.PARTIDAS || { itens: {} };
  const el = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text != null) e.textContent = text; return e; };
  const t = (tag, key, text, cls) => { const e = el(tag, cls, text); e.dataset.i18n = key; return e; };
  const cols = [["match.player", "Jogador", "col-player"], ["match.souls", "Almas", "num"], ["match.k", "K", "num kda"],
    ["match.d", "D", "num kda"], ["match.a", "A", "num kda"], ["match.dmg", "Dano", "num"],
    ["match.obj", "Obj.", "num"], ["match.heal", "Cura", "num"], ["match.items", "Itens", "col-items"]];

  const board = el("article", "match-board");
  const mh = el("div", "match-head");
  const label = el(titleTag, "match-label");
  label.append(t("span", "match.label", "Partida"), " " + numero);
  const d = new Date(P.inicio * 1000);
  const dur = Math.floor(P.duracao / 60) + ":" + String(P.duracao % 60).padStart(2, "0");
  const when = el("span", null, d.toLocaleDateString("pt-BR") + " · " + d.toLocaleTimeString("pt-BR", { hour: "2-digit", minute: "2-digit" }));
  when.dataset.datetime = d.getTime();
  const meta = el("p", "match-meta");
  meta.append(
    when,
    (() => { const s = el("span", null), i = el("span", "ico-time"); i.setAttribute("role", "img"); i.setAttribute("aria-label", "Duração"); i.dataset.i18nLabel = "match.duration"; s.append(i, dur); return s; })());
  if (P.id) meta.append(el("span", "match-id", "#" + P.id));
  mh.append(label, meta);
  board.appendChild(mh);

  P.times.forEach((team, ti) => {
    const win = P.vencedor === ti;
    const block = el("div", "match-team team-" + team.classe + (win ? " is-win" : ""));
    const head = el("div", "match-team-head");
    head.append(el("h4", null, team.nome),
      win ? t("span", "match.win", "Vitória", "match-badge") : t("span", "match.loss", "Derrota", "match-badge is-loss"),
      el("span", "match-total", team.almas));
    const scroll = el("div", "match-scroll");
    const table = el("table", "match-table");
    const hr = el("tr");
    cols.forEach(([k, txt, cls]) => { const th = t("th", k, txt, cls); th.scope = "col"; hr.appendChild(th); });
    table.appendChild(el("thead")).appendChild(hr);
    const body = table.appendChild(el("tbody"));
    team.jogadores.forEach((p) => {
      const isMvp = !!P.mvp && p.nome === P.mvp;   // MVP escolhido pelo ADM em partida.js
      const tr = el("tr", isMvp ? "is-mvp" : "");
      const who = el("td", "col-player"); const cell = el("div", "player");
      const img = el("img", "hero-ico"); img.src = p.imgUrl || "assets/partida/herois/" + p.img + ".webp"; img.alt = ""; img.width = 40; img.height = 40; img.loading = "lazy";
      const names = el("div", "who");
      const nm = el("strong", null, p.nome);
      if (isMvp) nm.append(" ", el("span", "mvp", "MVP"));
      names.append(nm, el("span", "hero-name", p.heroi));
      cell.append(img, names); who.appendChild(cell);
      const items = el("td", "col-items");
      const grid = el("div", "items");
      p.itens.forEach((s) => {
        const it = D.itens[s] || { nome: s, tipo: "weapon" };
        const tile = el("span", "mi mi-" + it.tipo + (it.ativo ? " is-active" : ""));
        tile.title = it.nome;
        const im = el("img"); im.src = it.imgUrl || "assets/partida/itens/" + s + ".webp"; im.alt = it.nome; im.width = 34; im.height = 34; im.loading = "lazy";
        tile.appendChild(im); grid.appendChild(tile);
      });
      items.appendChild(grid);
      tr.append(who, el("td", "num", p.almas), el("td", "num kda k", p.k), el("td", "num kda d", p.d), el("td", "num kda", p.a),
        el("td", "num", p.dano), el("td", "num", p.obj), el("td", "num", p.cura), items);
      body.appendChild(tr);
    });
    scroll.appendChild(table);
    block.append(head, scroll);
    board.appendChild(block);
  });
  return board;
};
