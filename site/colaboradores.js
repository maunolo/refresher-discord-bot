// Colaboradores mostrados na faixa do rodapé (index.html e historico.html).
// Para editar: troque, adicione ou remova linhas. "funcao" e "link" são opcionais.
// Se a lista ficar vazia, a faixa some do site.
const COLABORADORES = [
  { nome: "Hydan_g" },
  { nome: "Haunterwell" },
  { nome: "Maunolo" },
  { nome: "Pinkish_nana" },
  { nome: "Aqua" },
  { nome: "Atlas_sw" },
  { nome: "Rique" },
];

(function () {
  const strip = document.getElementById("colaboradores");
  if (!strip) return;
  if (!COLABORADORES.length) { strip.hidden = true; return; }
  const list = strip.querySelector(".collab-list");
  COLABORADORES.forEach((c) => {
    const li = document.createElement("li");
    li.className = "collab";
    const name = document.createElement(c.link ? "a" : "span");
    name.className = "collab-name";
    name.textContent = c.nome;
    if (c.link) { name.href = c.link; name.target = "_blank"; name.rel = "noopener"; }
    li.appendChild(name);
    if (c.funcao) {
      const role = document.createElement("span");
      role.className = "collab-role";
      role.textContent = c.funcao;
      li.appendChild(role);
    }
    list.appendChild(li);
  });
})();
