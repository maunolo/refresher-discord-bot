import { useActions } from '../actions';
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

/** Both slots free: between matches, or done for the day. */
function NoOpenMatch({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  if (camp.status === 'ended') {
    return (
      <section className="card empty-card">
        <h2>Camp encerrado</h2>
        <p className="muted small">O ranking final foi publicado no Discord.</p>
        <a className="button button-primary" href={routeHref('ranking')}>
          Ver ranking final
        </a>
      </section>
    );
  }
  if (camp.status !== 'running') {
    return (
      <section className="card empty-card">
        <h2>Nenhuma partida aberta</h2>
        <p className="muted small">Este camp não está em andamento.</p>
      </section>
    );
  }
  const canEnd = camp.match_count > 0;
  return (
    <section className="card empty-card">
      <h2>Nenhuma partida aberta</h2>
      <p className="muted small">
        Crie uma partida para tirar os próximos jogadores da fila
        {canEnd ? ', ou encerre o camp se o dia acabou.' : '.'}
      </p>
      {canEnd && (
        <div className="form-actions">
          <button
            type="button"
            className="button button-danger"
            disabled={busy}
            onClick={() =>
              run(
                { type: 'end_camp' },
                {
                  confirm: {
                    title: `Encerrar o camp ${camp.id}?`,
                    message: `O ranking final é publicado no Discord depois de ${
                      camp.match_count === 1 ? '1 partida' : `${camp.match_count} partidas`
                    }, e o camp termina.`,
                    note: 'Não dá para desfazer.',
                    action: 'Encerrar camp',
                    tone: 'danger',
                  },
                  then: () => {
                    window.location.hash = routeHref('ranking');
                  },
                },
              )
            }
          >
            Encerrar camp
          </button>
        </div>
      )}
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
            <NoOpenMatch camp={camp} />
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
