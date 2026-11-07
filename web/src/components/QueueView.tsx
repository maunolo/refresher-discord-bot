import { useState, type DragEvent } from 'react';

import { useActions } from '../actions';
import type { Camp } from '../api';
import { playerById, playerName, points, queuedPlayers } from '../camp';
import { routeHref } from '../route';
import { DiscordLink, RoleChip, SRankChip } from './common';
import { ArrowDownIcon, ArrowUpIcon, GripIcon, ToTopIcon } from './Icons';

function sameMembers(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  const set = new Set(a);
  return b.every((id) => set.has(id));
}

function moved(order: string[], from: number, to: number): string[] {
  const next = [...order];
  const [id] = next.splice(from, 1);
  next.splice(to, 0, id);
  return next;
}

/** The queue in order, with ↑ ↓ and drag to change it. Changes stay on the page until saved. */
export function QueueView({ camp }: { camp: Camp }) {
  const { run, busy } = useActions();
  const serverOrder = queuedPlayers(camp).map((p) => p.user_id);
  const [draft, setDraft] = useState<string[] | null>(null);
  const [dragged, setDragged] = useState<string | null>(null);
  const [dropAt, setDropAt] = useState<number | null>(null);

  if (camp.status !== 'running') {
    return (
      <section className="card empty-card">
        <h2>Sem fila agora</h2>
        <p className="muted small">
          {camp.status === 'signup'
            ? 'A fila começa quando a inscrição fecha. Monte o elenco e inicie a fila em Jogadores.'
            : 'A fila só existe com o camp em andamento.'}
        </p>
        {camp.status === 'signup' && (
          <a className="button button-primary" href={routeHref('players')}>
            Abrir Jogadores
          </a>
        )}
      </section>
    );
  }

  const order = draft ?? serverOrder;
  const editing = draft !== null && draft.join() !== serverOrder.join();
  // A match started or ended while editing: the saved order would drop or add someone.
  const stale = draft !== null && !sameMembers(draft, serverOrder);
  const upNext = new Set(camp.up_next);
  const missing = camp.match_size - serverOrder.length;
  const move = (from: number, to: number) => {
    if (to < 0 || to >= order.length || from === to) return;
    setDraft(moved(order, from, to));
  };
  const save = () => run({ type: 'reorder_queue', user_ids: order }, { then: () => setDraft(null) });

  const onDragStart = (e: DragEvent, id: string) => {
    setDragged(id);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', id);
  };
  const onDragOver = (e: DragEvent, index: number) => {
    if (!dragged) return;
    e.preventDefault();
    const box = e.currentTarget.getBoundingClientRect();
    setDropAt(e.clientY > box.top + box.height / 2 ? index + 1 : index);
  };
  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    if (dragged !== null && dropAt !== null) {
      const from = order.indexOf(dragged);
      move(from, dropAt > from ? dropAt - 1 : dropAt);
    }
    setDragged(null);
    setDropAt(null);
  };

  return (
    <div className="split">
      <section className="split-main card" aria-labelledby="queue-page-title">
        <div className="card-head">
          <div className="title-row">
            <h1 id="queue-page-title" className="page-title">
              Fila
            </h1>
            <span className="mono muted small">
              {serverOrder.length} {serverOrder.length === 1 ? 'jogador' : 'jogadores'}
            </span>
          </div>
          <div className="inline-actions">
            {draft !== null && (
              <button type="button" className="button button-ghost" disabled={busy} onClick={() => setDraft(null)}>
                Descartar
              </button>
            )}
            <button
              type="button"
              className="button button-primary"
              disabled={busy || !editing || stale}
              onClick={save}
            >
              Salvar ordem
            </button>
          </div>
        </div>
        {stale ? (
          <p className="notice-inline" role="alert">
            A fila mudou enquanto você reordenava (uma partida começou ou terminou). Descarte e reordene de novo.
          </p>
        ) : editing ? (
          <p className="muted small" role="status">
            Ordem alterada, ainda não salva. O Discord só muda quando você salvar.
          </p>
        ) : (
          <p className="muted small">
            {missing > 0
              ? `Uma partida precisa de ${camp.match_size}. Faltam ${missing}; os próximos entram quando uma partida terminar.`
              : 'Marcados com → entram na próxima partida: os primeiros da fila.'}
          </p>
        )}
        {order.length === 0 ? (
          <p className="empty">Todo mundo está em partida.</p>
        ) : (
          <ol className="rows reorder" onDragLeave={() => setDropAt(null)}>
            {order.map((id, i) => {
              const player = playerById(camp, id);
              const name = playerName(camp, id);
              const classes = ['row', 'row-tall', 'reorder-row'];
              if (dragged === id) classes.push('reorder-dragging');
              if (dropAt === i) classes.push('reorder-drop-before');
              if (dropAt === i + 1 && i === order.length - 1) classes.push('reorder-drop-after');
              return (
                <li
                  key={id}
                  className={classes.join(' ')}
                  draggable={!busy}
                  onDragStart={(e) => onDragStart(e, id)}
                  onDragOver={(e) => onDragOver(e, i)}
                  onDrop={onDrop}
                  onDragEnd={() => {
                    setDragged(null);
                    setDropAt(null);
                  }}
                >
                  <span className="grip" aria-hidden="true">
                    <GripIcon size={16} />
                  </span>
                  <span className="mono faint small queue-pos">{String(i + 1).padStart(2, '0')}</span>
                  <span className="up-next" title={draft === null && missing <= 0 && upNext.has(id) ? 'Entra na próxima partida' : undefined}>
                    {draft === null && missing <= 0 && upNext.has(id) ? '→' : ''}
                  </span>
                  <span className="ellipsis">{name}</span>
                  {player?.s_rank && <SRankChip />}
                  <RoleChip role={player?.role ?? null} />
                  <span className="row-end muted small queue-seg">{player?.consecutive_matches ?? 0} seg.</span>
                  <span className="mono small points">{points(player?.points ?? 0)}</span>
                  <span className="reorder-buttons">
                    <button
                      type="button"
                      className="icon-button"
                      aria-label={`Mover ${name} para o topo`}
                      title="Topo"
                      disabled={busy || i === 0}
                      onClick={() => move(i, 0)}
                    >
                      <ToTopIcon size={15} />
                    </button>
                    <button
                      type="button"
                      className="icon-button"
                      aria-label={`Subir ${name}`}
                      title="Subir"
                      disabled={busy || i === 0}
                      onClick={() => move(i, i - 1)}
                    >
                      <ArrowUpIcon size={15} />
                    </button>
                    <button
                      type="button"
                      className="icon-button"
                      aria-label={`Descer ${name}`}
                      title="Descer"
                      disabled={busy || i === order.length - 1}
                      onClick={() => move(i, i + 1)}
                    >
                      <ArrowDownIcon size={15} />
                    </button>
                  </span>
                </li>
              );
            })}
          </ol>
        )}
      </section>
      <aside className="split-aside">
        <section className="card" aria-labelledby="queue-rules-title">
          <h2 id="queue-rules-title">Como a fila anda</h2>
          <p className="muted small">
            A fila começa sorteada em times, com os S rank divididos por igual. Uma partida nova pega os primeiros da fila,
            S rank ou não. Depois de cada resultado, quem
            estava esperando continua na frente, nesta ordem; quem acabou de jogar entra no fim, quem jogou menos partidas
            seguidas primeiro e vencedores antes dos perdedores.
          </p>
          <p className="muted small">Mude a ordem para corrigir um atraso ou dar a vez a quem precisa sair mais cedo.</p>
        </section>
        <section className="card card-dashed" aria-labelledby="queue-discord-title">
          <h2 id="queue-discord-title" className="small-title">
            No Discord
          </h2>
          <p className="muted small">A mensagem do camp mostra a fila em ordem. Ela é atualizada quando você salva.</p>
          <DiscordLink url={camp.discord_url} label="Abrir a mensagem da fila" />
        </section>
      </aside>
    </div>
  );
}
