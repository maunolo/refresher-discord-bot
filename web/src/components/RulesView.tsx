/** How the bot picks who plays and makes the teams, written out for admins and players. */
export function RulesView() {
  return (
    <section className="stack rules" aria-labelledby="rules-title">
      <div className="title-row">
        <h1 id="rules-title" className="page-title">
          Regras da fila e das partidas
        </h1>
      </div>
      <p className="muted">
        Como o bot escolhe quem joga e como monta os times. Os números são de um camp 6v6; nos testes, o 2v2 leva 4
        jogadores e o refresh precisa de 2 votos, e o 1v1 leva 2 jogadores e precisa de 1 voto.
      </p>

      <section className="card" aria-labelledby="rules-played">
        <h2 id="rules-played">Partidas jogadas</h2>
        <p>
          O número ao lado de cada jogador na fila conta as partidas terminadas que ele jogou no camp. Quem jogou menos joga
          primeiro, e ele também desempata o ranking, depois dos MVPs e antes das vitórias. Uma partida cancelada ou reaberta não conta.
        </p>
      </section>

      <section className="card" aria-labelledby="rules-ranks">
        <h2 id="rules-ranks">Categorias</h2>
        <ul>
          <li>
            <strong>SS rank:</strong> os capitães. Todo SS rank de um time é capitão e recebe o link do draft, e um time de
            uma partida tem no máximo dois.
          </li>
          <li>
            <strong>S rank:</strong> os jogadores fortes, divididos por igual entre os times.
          </li>
          <li>Cada jogador tem uma categoria ou nenhuma. A categoria não dá prioridade na fila.</li>
        </ul>
      </section>

      <section className="card" aria-labelledby="rules-start">
        <h2 id="rules-start">1. Início da fila</h2>
        <p>
          <strong>Iniciar fila</strong> precisa de jogadores para uma partida (12); o elenco vai até 24 ou 30, e quem
          faltar pode entrar depois. O elenco pode vir de uma{' '}
          <strong>seleção</strong> salva, com categorias e funções prontas. Ele é sorteado numa fila de blocos de 6, meia
          partida cada:
        </p>
        <ul>
          <li>
            Primeiro os SS rank, depois os S rank, divididos o mais por igual possível entre os blocos: um bloco com um SS a
            menos recebe um S antes. Quais blocos ganham um a mais é sorteado.
          </li>
          <li>
            Depois as funções, da mais rara para a mais comum: cada jogador vai para o bloco com menos jogadores da sua
            função. Assim, cada bloco tem um Tank/Bruiser, um Carry e um Utility/Support quando o elenco permite, e as
            primeiras partidas começam com pelo menos dois de cada.
          </li>
          <li>O resto é sorteado. Todos começam com 0 partidas jogadas.</li>
          <li>
            Quem entra com a fila andando vai para o fim dela, com 0 pontos e 0 partidas: por ter jogado menos, é sorteado
            antes dos outros.
          </li>
        </ul>
        <p>A primeira partida pega os dois primeiros blocos, a segunda os dois seguintes.</p>
      </section>

      <section className="card" aria-labelledby="rules-pick">
        <h2 id="rules-pick">2. Quem joga uma partida</h2>
        <p>
          Fora da final, <strong>Criar partida</strong> precisa de uma vaga livre (no máximo duas partidas ao mesmo tempo)
          e de pelo menos 12 jogadores na fila. Ela pega <strong>os 12 da fila que jogaram menos partidas</strong>; entre
          quem jogou o mesmo tanto, a ordem da fila. S rank, função e pontos não dão prioridade. A → no quadro marca esses
          12.
        </p>
        <p>
          Só cabem quatro SS rank, dois por time: um quinto espera a próxima partida, a não ser que sem ele não dê para
          fechar os 12.
        </p>
        <p>
          Os admins podem reordenar a fila à mão em <strong>Fila</strong>; a ordem vale entre quem jogou o mesmo número de
          partidas, até o próximo resultado. Ao criar a partida, os jogadores vão para os canais de voz dos seus times.
        </p>
      </section>

      <section className="card" aria-labelledby="rules-split">
        <h2 id="rules-split">3. Divisão dos times</h2>
        <p>
          O bot testa todas as formas de dividir os 12 ao meio (924) e fica com a melhor, por estas regras, nesta ordem:
        </p>
        <ol>
          <li>
            <strong>Categorias:</strong> a quantidade de SS rank nos dois times difere em no máximo 1 (com quatro, dois e
            dois), depois a de S rank, e depois as duas somadas.
          </li>
          <li>
            <strong>Funções:</strong> o menor número de funções faltando nos dois times; no empate, ganha a divisão em que
            falta menos no time mais prejudicado. Com dois de cada função na partida, os dois times ficam com um de cada.
            Com um Tank/Bruiser e um Utility/Support só, cada time fica com um dos dois.
          </li>
          <li>
            <strong>Só na final:</strong> os totais de pontos mais próximos.
          </li>
          <li>
            <strong>No resto, sorteio</strong>, inclusive qual time é Hiddenking e qual é Archmother.
          </li>
        </ol>
        <p>
          <strong>Capitães:</strong> todo SS rank de um time é capitão. Um time sem SS rank tem um capitão: o S rank com
          mais pontos, ou, sem S rank, quem tem mais pontos, com sorteio no empate.
        </p>
      </section>

      <section className="card" aria-labelledby="rules-vote">
        <h2 id="rules-vote">4. Votação de refresh</h2>
        <ul>
          <li>
            Cada jogador da partida vota uma vez no Discord: <strong>Refresh</strong> ou <strong>Manter times</strong>, de
            dentro do canal de voz do seu time.
          </li>
          <li>
            <strong>Passa</strong> assim que a partida tem 6 votos Refresh, somando os dois times.
          </li>
          <li>
            <strong>Os times ficam</strong> quando todos votaram sem chegar a 6, ou quando o refresh não tem mais como
            passar.
          </li>
          <li>
            Os admins podem encerrar antes: <strong>Fechar e manter times</strong> mantém os times, e{' '}
            <strong>Sortear de novo</strong> age como se o refresh tivesse passado.
          </li>
          <li>Cada partida só vota uma vez.</li>
        </ul>
      </section>

      <section className="card" aria-labelledby="rules-refresh">
        <h2 id="rules-refresh">5. O que o refresh muda</h2>
        <p>Os 12 jogadores da partida e os da fila entram juntos, por partidas jogadas, quem jogou menos primeiro:</p>
        <ul>
          <li>
            Quem jogou menos <strong>fica com a vaga</strong>, ou <strong>entra da fila</strong> no lugar de quem jogou
            mais.
          </li>
          <li>
            As últimas vagas, entre quem jogou o mesmo tanto, são <strong>sorteadas</strong> entre os jogadores da partida e
            os da fila.
          </li>
          <li>Continuam valendo no máximo dois SS rank por time.</li>
          <li>
            Quem perde a vaga volta para a fila, na ordem de partidas jogadas, e para o canal de voz Queue; quem entra vai
            para o canal do seu time.
          </li>
          <li>Os 12 novos são divididos pelas regras da divisão dos times.</li>
          <li>
            Se ninguém da fila jogou menos, o refresh só reembaralha os mesmos 12, ou sorteia as vagas entre a partida e
            quem está empatado com ela.
          </li>
          <li>
            A <strong>final só reembaralha</strong> os próprios jogadores.
          </li>
        </ul>
      </section>

      <section className="card" aria-labelledby="rules-result">
        <h2 id="rules-result">6. Depois de um resultado</h2>
        <p>
          Cada jogador ganha os pontos que a partida vale: o admin escolhe ao criá-la quanto vale a vitória e quanto vale a
          derrota (1 e 0, se ninguém mudar). O MVP é escolhido entre os vencedores. Todos que jogaram ganham +1 partida
          jogada. Aí a fila é refeita:
        </p>
        <ol>
          <li>Em ordem de partidas jogadas no camp, quem jogou menos na frente.</li>
          <li>Com o mesmo número, quem estava esperando vem primeiro, na ordem em que estava.</li>
          <li>
            Depois, <strong>vencedores antes dos perdedores</strong>, então quem vence joga de novo mais cedo.
          </li>
          <li>Um grupo com mais de 2 vencedores (ou perdedores) é embaralhado.</li>
        </ol>
        <p>Todos da partida voltam para o canal de voz Queue.</p>
      </section>

      <section className="card" aria-labelledby="rules-ranking">
        <h2 id="rules-ranking">7. O ranking</h2>
        <p>
          Por pontos. No empate, fica na frente quem tem <strong>mais MVPs</strong>; depois, quem{' '}
          <strong>jogou menos partidas</strong>; por fim, quem tem <strong>mais vitórias</strong>. Empatados nos quatro
          dividem o lugar.
        </p>
      </section>

      <section className="card" aria-labelledby="rules-final">
        <h2 id="rules-final">8. A final</h2>
        <p>
          <strong>Entrar na final</strong> exige pelo menos uma partida já pontuada. A partir daí, cada partida criada pega{' '}
          <strong>os 12 primeiros do ranking</strong> (pontos, depois MVPs, menos partidas e mais vitórias) em vez da fila, e a → no
          quadro marca esses 12.
        </p>
        <ul>
          <li>
            O ranking é visto de novo a cada partida: ajustes de pontos e jogadores removidos contam. No empate nos quatro
            pela última vaga, fica quem jogou a última partida da final; depois, a ordem da fila.
          </li>
          <li>
            Os times são divididos pelas regras da divisão, incluindo os pontos. Os 12 do ranking jogam mesmo que tenham mais
            de quatro SS rank; aí os SS ficam o mais divididos possível.
          </li>
          <li>Uma partida por vez, quantas forem precisas. Cancelar uma delas não tira o camp da final.</li>
          <li>
            <strong>Sair da final</strong> volta a criar as partidas pela fila, desde que nenhuma partida da final esteja
            aberta. <strong>Encerrar camp</strong> posta o ranking.
          </li>
        </ul>
      </section>

      <section className="card" aria-labelledby="rules-example">
        <h2 id="rules-example">Exemplo com 30 jogadores</h2>
        <div className="table-scroll">
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Momento</th>
                <th scope="col">Fila (da frente para trás)</th>
                <th scope="col">Partidas</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Iniciar fila</td>
                <td>blocos A B C D E, todos com 0 partidas</td>
                <td>—</td>
              </tr>
              <tr>
                <td>Duas partidas criadas</td>
                <td>E (0)</td>
                <td>P1 = A+B, P2 = C+D</td>
              </tr>
              <tr>
                <td>P1 termina, A vence</td>
                <td>E (0) → A (1, venceu) → B (1, perdeu)</td>
                <td>P2 ao vivo</td>
              </tr>
              <tr>
                <td>Partida criada</td>
                <td>B (1)</td>
                <td>P3 = E+A</td>
              </tr>
              <tr>
                <td>Refresh da P3</td>
                <td colSpan={2}>
                  E (0) fica com as suas 6 vagas; as 6 de A são sorteadas entre A e B, todos com 1 (12 jogadores). Quem perde
                  a vaga volta para a fila com o resto de B.
                </td>
              </tr>
              <tr>
                <td>P2 termina, C vence</td>
                <td>B (1, esperou) → C (1, venceu) → D (1, perdeu)</td>
                <td>—</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}
