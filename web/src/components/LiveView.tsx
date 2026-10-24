import type { Camp } from '../api';
import { playerName } from '../camp';
import { MatchCard } from './MatchCard';
import { HistoryPanel, QueuePanel, RankingPanel } from './Panels';
import { SRankChip } from './common';

function SignupView({ camp }: { camp: Camp }) {
  const roster = [...camp.players].sort((a, b) => playerName(camp, a.user_id).localeCompare(playerName(camp, b.user_id)));
  return (
    <section className="card" aria-labelledby="signup-title">
      <div className="card-head">
        <h2 id="signup-title">Inscrição</h2>
        <span className="mono muted small">
          {camp.players.length}/{camp.target_size}
        </span>
      </div>
      <p className="muted small">
        Montar o elenco e iniciar a fila pelo painel chega na próxima etapa. Por enquanto, use <span className="mono">/camp</span> no Discord.
      </p>
      {roster.length === 0 ? (
        <p className="empty">Ninguém no elenco ainda.</p>
      ) : (
        <ul className="rows roster">
          {roster.map((p) => (
            <li key={p.user_id} className="row row-tall">
              <span className="ellipsis">{playerName(camp, p.user_id)}</span>
              {p.s_rank && <SRankChip />}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export function LiveView({ camp }: { camp: Camp }) {
  if (camp.status === 'signup') return <SignupView camp={camp} />;
  return (
    <div className="live">
      <div className="live-top">
        <div className="matches">
          {camp.open_matches.length === 0 ? (
            <section className="card empty-card">
              <h2>Nenhuma partida aberta</h2>
              <p className="muted small">
                {camp.status === 'running'
                  ? 'Crie uma partida para tirar os próximos jogadores da fila.'
                  : 'Este camp não está em andamento.'}
              </p>
            </section>
          ) : (
            camp.open_matches.map((m) => <MatchCard key={m.id} camp={camp} match={m} />)
          )}
        </div>
        <QueuePanel camp={camp} />
      </div>
      <div className="live-bottom">
        <HistoryPanel camp={camp} />
        <RankingPanel camp={camp} />
      </div>
    </div>
  );
}
