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

      <section className="card" aria-labelledby="rules-streak">
        <h2 id="rules-streak">Partidas seguidas</h2>
        <p>
          O "seg." no quadro conta quantas partidas o jogador jogou sem ficar fora de nenhuma. Sobe 1 para todos da partida
          quando sai o resultado, e volta a 0 para todos que estavam esperando na fila quando outra partida termina.
        </p>
      </section>

      <section className="card" aria-labelledby="rules-start">
        <h2 id="rules-start">1. Início da fila</h2>
        <p>
          <strong>Iniciar fila</strong> exige o elenco exatamente completo (24 ou 30). O elenco é sorteado numa fila de
          blocos de 6, meia partida cada:
        </p>
        <ul>
          <li>
            Primeiro os S rank, divididos o mais por igual possível entre os blocos. Quais blocos ganham um a mais é
            sorteado.
          </li>
          <li>
            Depois as funções, da mais rara para a mais comum: cada jogador vai para o bloco com menos jogadores da sua
            função. Assim, cada bloco tem um Tank/Bruiser, um Carry e um Utility/Support quando o elenco permite, e as
            primeiras partidas começam com pelo menos dois de cada.
          </li>
          <li>O resto é sorteado. Todos começam com 0 partidas seguidas.</li>
        </ul>
        <p>A primeira partida pega os dois primeiros blocos, a segunda os dois seguintes.</p>
      </section>

      <section className="card" aria-labelledby="rules-pick">
        <h2 id="rules-pick">2. Quem joga uma partida</h2>
        <p>
          <strong>Criar partida</strong> precisa de uma vaga livre (no máximo duas partidas ao mesmo tempo), da final
          ainda não criada e de pelo menos 12 jogadores na fila. Ela pega <strong>os 12 primeiros da fila</strong>, e mais
          nada conta: S rank, função e pontos não dão prioridade. A → no quadro marca esses 12.
        </p>
        <p>
          Os admins podem reordenar a fila à mão em <strong>Fila</strong>; essa ordem vale até o próximo resultado. Ao criar
          a partida, os jogadores vão para os canais de voz dos seus times.
        </p>
      </section>

      <section className="card" aria-labelledby="rules-split">
        <h2 id="rules-split">3. Divisão dos times</h2>
        <p>
          O bot testa todas as formas de dividir os 12 ao meio (924) e fica com a melhor, por estas regras, nesta ordem:
        </p>
        <ol>
          <li>
            <strong>S rank:</strong> a quantidade de S rank nos dois times difere em no máximo 1. Isso vale sempre.
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
          <strong>Capitães:</strong> todo S rank de um time é capitão. Um time sem S rank tem como capitão quem tem mais
          pontos, com sorteio no empate.
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
            <strong>Passa</strong> assim que um dos lados tem 4 votos Refresh.
          </li>
          <li>
            <strong>Os times ficam</strong> quando todos votaram sem nenhum lado chegar a 4, ou quando o refresh não tem
            mais como passar.
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
        <p>Seja H o maior número de partidas seguidas entre os 12 jogadores da partida.</p>
        <ul>
          <li>
            Quem está <strong>abaixo de H fica com a vaga</strong>: estava esperando.
          </li>
          <li>
            As outras vagas são <strong>sorteadas</strong> entre os jogadores da partida com H e os da fila com H ou menos.
            Ninguém que jogou mais partidas seguidas entra.
          </li>
          <li>
            Quem perde a vaga vai para o <strong>começo da fila</strong> e para o canal de voz Queue; quem entra vai para o
            canal do seu time.
          </li>
          <li>Os 12 novos são divididos pelas regras da divisão dos times.</li>
          <li>
            Se ninguém da fila se encaixa, o refresh só reembaralha os mesmos 12. No começo do camp todos têm 0, então a
            partida inteira é sorteada de novo entre ela e a fila.
          </li>
          <li>
            A <strong>final só reembaralha</strong> os próprios jogadores.
          </li>
        </ul>
      </section>

      <section className="card" aria-labelledby="rules-result">
        <h2 id="rules-result">6. Depois de um resultado</h2>
        <p>Os vencedores ganham +1 ponto, e todos que jogaram ganham +1 partida seguida. Aí a fila é refeita:</p>
        <ol>
          <li>Quem estava esperando zera e continua na frente, na ordem em que estava.</li>
          <li>Atrás vêm os jogadores da partida, com menos partidas seguidas primeiro.</li>
          <li>
            Com o mesmo número, <strong>vencedores vêm antes dos perdedores</strong>, então quem vence joga de novo mais
            cedo.
          </li>
          <li>Um grupo com mais de 2 vencedores (ou perdedores) é embaralhado.</li>
        </ol>
        <p>Todos da partida voltam para o canal de voz Queue.</p>
      </section>

      <section className="card" aria-labelledby="rules-final">
        <h2 id="rules-final">7. A final</h2>
        <p>
          <strong>Partida final</strong> exige nenhuma partida aberta e pelo menos uma já pontuada. Ela pega{' '}
          <strong>os 12 com mais pontos</strong>, com sorteio nos empates pelas últimas vagas, e não olha a fila. Os times
          são divididos pelas regras da divisão, incluindo os pontos. Depois dela não dá para criar outra partida, e{' '}
          <strong>Encerrar camp</strong> posta o ranking.
        </p>
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
                <td>blocos A B C D E, todos com 0 seguidas</td>
                <td>—</td>
              </tr>
              <tr>
                <td>Duas partidas criadas</td>
                <td>E (0)</td>
                <td>P1 = A+B, P2 = C+D</td>
              </tr>
              <tr>
                <td>P1 termina, A vence</td>
                <td>E (0, esperou) → A (1, venceu) → B (1, perdeu)</td>
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
                  H = 1. E fica com as suas 6 vagas; as 6 de A são sorteadas entre A e B (12 jogadores). Quem perde a vaga
                  vai para a frente do resto de B.
                </td>
              </tr>
              <tr>
                <td>P2 termina, C vence</td>
                <td>os de B zeram → C (1) → D (1)</td>
                <td>—</td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
    </section>
  );
}
