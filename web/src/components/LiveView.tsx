import type { Camp } from '../api';
import { routeHref } from '../route';
import { MatchCard } from './MatchCard';
import { HistoryPanel, QueuePanel, RankingPanel } from './Panels';

function SignupNote({ camp }: { camp: Camp }) {
  return (
    <section className="card empty-card">
      <h2>Inscrição aberta</h2>
      <p className="muted small">
        {camp.players.length} de {camp.target_size} jogadores. Monte o elenco e inicie a fila em Jogadores; as partidas
        aparecem aqui depois.
      </p>
      <a className="button button-primary" href={routeHref('players')}>
        Abrir Jogadores
      </a>
    </section>
  );
}

export function LiveView({ camp }: { camp: Camp }) {
  if (camp.status === 'signup') return <SignupNote camp={camp} />;
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
