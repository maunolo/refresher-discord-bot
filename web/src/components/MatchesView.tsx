import { useEffect, useState, type ReactNode } from 'react';

import { confirmCloseVote, confirmRefreshTeams, useActions } from '../actions';
import { api, type Camp, type Match, type Side } from '../api';
import {
  SIDES,
  SIDE_LABEL,
  draftStarted,
  matchName,
  playerById,
  playerName,
  points,
  stage,
  voteCount,
  voteSummary,
} from '../camp';
import { pollEvery, useLive } from '../live';
import { matchHref, routeHref } from '../route';
import { usePolling } from '../usePolling';
import { Captain, CopyButton, DiscordLink, MvpTag, RoleChip, SRankChip } from './common';
import { DraftLinks, ManualDraftNote, StagePill, Stepper } from './MatchCard';
import { Corrections } from './Panels';
import { WinnerDialog } from './ResultDialogs';

const loadMatches = (campId: string) => api.matches(campId);

function finishedAt(seconds: number): string {
  return new Date(seconds * 1000).toLocaleString('pt-BR', {
    day: '2-digit',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function title(match: Match): string {
  if (match.number === null) return matchName(match);
  return match.is_final ? `#${match.number} · final` : `#${match.number}`;
}

/** Open matches by slot, finished ones by their number. */
function detailName(match: Match): string {
  if (match.number === null) return matchName(match);
  return match.is_final ? `Partida final (#${match.number})` : `Partida #${match.number}`;
}

/** The camp's matches, or one of them when the URL names it. */
export function MatchesView({ camp, matchId }: { camp: Camp; matchId: string | null }) {
  const live = useLive();
  const matches = usePolling<Match[]>(camp.id, loadMatches, pollEvery(live, 15_000), live.version);
  const refresh = matches.refresh;
  // The camp reloads every few seconds and after each action; the list follows it.
  useEffect(() => refresh(), [camp, refresh]);

  if (matchId) {
    const open = camp.open_matches.find((m) => m.id === matchId);
    const listed = matches.data?.find((m) => m.id === matchId);
    const match = open ?? listed;
    if (match) return <MatchDetail camp={camp} match={match} />;
    return (
      <section className="card empty-card">
        <h2>{matches.data ? 'Partida não encontrada' : 'Carregando…'}</h2>
        {matches.data && (
          <p className="muted small">
            A partida <span className="mono">{matchId}</span> não é deste camp.
          </p>
        )}
        <a className="button" href={routeHref('matches')}>
          Ver todas as partidas
        </a>
      </section>
    );
  }

  const list = matches.data ?? [];
  return (
    <section className="stack" aria-labelledby="matches-title">
      <div className="card-head">
        <h1 id="matches-title" className="page-title">
          Partidas
        </h1>
        <span className="muted small">
          {camp.match_count} {camp.match_count === 1 ? 'terminada' : 'terminadas'} · {camp.open_matches.length}{' '}
          {camp.open_matches.length === 1 ? 'aberta' : 'abertas'}
        </span>
      </div>
      {matches.error && !matches.data && <p className="error small">{matches.error.message}</p>}
      {!matches.data && !matches.error && <p className="muted">Carregando…</p>}
      {matches.data && list.length === 0 && (
        <section className="card empty-card">
          <h2>Nenhuma partida ainda</h2>
          <p className="muted small">
            {camp.status === 'running'
              ? 'Crie a primeira partida no topo da página.'
              : 'As partidas aparecem aqui depois que a fila começa.'}
          </p>
        </section>
      )}
      {list.length > 0 && (
        <div className="card table-scroll">
          <table className="table">
            <thead>
              <tr>
                <th scope="col">Partida</th>
                <th scope="col">ID</th>
                <th scope="col">Etapa</th>
                <th scope="col">Vencedor</th>
                <th scope="col">Capitães</th>
                <th scope="col">MVP</th>
                <th scope="col">Terminou</th>
                <th scope="col" className="align-end">
                  <span className="visually-hidden">Links</span>
                </th>
              </tr>
            </thead>
            <tbody>
              {list.map((m) => (
                <tr key={m.id}>
                  <td>
                    <a href={matchHref(m.id)}>{title(m)}</a>
                  </td>
                  <td className="mono">{m.id}</td>
                  <td>
                    <StagePill match={m} />
                  </td>
                  <td>
                    {m.winner_side ? (
                      <span className={`tag tag-${m.winner_side}`}>{SIDE_LABEL[m.winner_side]}</span>
                    ) : (
                      <span className="faint">—</span>
                    )}
                  </td>
                  <td className="muted">
                    {m.players
                      .filter((p) => p.is_captain)
                      .map((p) => playerName(camp, p.user_id))
                      .join(' · ') || '—'}
                  </td>
                  <td>{m.mvp_user_id ? playerName(camp, m.mvp_user_id) : <span className="faint">—</span>}</td>
                  <td className="muted">{m.completed_at ? finishedAt(m.completed_at) : '—'}</td>
                  <td className="align-end">
                    <DiscordLink url={m.discord_url} label="Discord" />
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}

function Stat({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="stat">
      <span className="eyebrow">{label}</span>
      {children}
    </div>
  );
}

function Meter({ value, of, tone, label }: { value: number; of: number; tone: string; label: string }) {
  const width = of > 0 ? Math.min(100, (value / of) * 100) : 0;
  return (
    <div className="meter" role="img" aria-label={label}>
      <div className={`meter-fill meter-${tone}`} style={{ width: `${width}%` }} />
    </div>
  );
}

function VoteTiles({ camp, match }: { camp: Camp; match: Match }) {
  const count = voteCount(camp, match);
  return (
    <div className="stats">
      {SIDES.map((side) => (
        <Stat key={side} label={`Refresh · ${SIDE_LABEL[side]}`}>
          <span className={`stat-value stat-${side}`}>
            {count.refresh[side]}
            <span className="stat-of"> de {count.needed}</span>
          </span>
          <Meter
            value={count.refresh[side]}
            of={count.needed}
            tone={side}
            label={`${count.refresh[side]} de ${count.needed} votos`}
          />
        </Stat>
      ))}
      <Stat label="Manter times">
        <span className="stat-value">{count.keep}</span>
        <span className="muted small">votos para não mudar</span>
      </Stat>
      <Stat label="Votaram">
        <span className="stat-value">
          {count.voted}
          <span className="stat-of"> de {count.total}</span>
        </span>
        <Meter value={count.voted} of={count.total} tone="jade" label={`${count.voted} de ${count.total} jogadores votaram`} />
      </Stat>
    </div>
  );
}

function VoteTag({ match, userId }: { match: Match; userId: string }) {
  const vote = match.votes.find((v) => v.user_id === userId);
  if (vote) return vote.refresh ? <span className="vote vote-refresh">Refresh</span> : <span className="vote vote-keep">Manter</span>;
  if (match.refresh_used) return null;
  return <span className="vote vote-waiting">Aguardando</span>;
}

function TeamVotes({ camp, match, side }: { camp: Camp; match: Match; side: Side }) {
  const team = match.players
    .filter((p) => p.side === side)
    .sort((a, b) => Number(b.is_captain) - Number(a.is_captain));
  return (
    <div className="team">
      <h3 className={`team-title team-${side}`}>
        <span className="swatch" />
        {SIDE_LABEL[side]}
        {match.winner_side === side && <span className="won">Venceu</span>}
      </h3>
      <ul className="rows">
        {team.map((p) => {
          const player = playerById(camp, p.user_id);
          return (
            <li key={p.user_id} className="row row-tall">
              <span className="ellipsis">{playerName(camp, p.user_id)}</span>
              {player?.s_rank && <SRankChip />}
              <RoleChip role={player?.role ?? null} />
              {p.is_captain && <Captain />}
              {match.mvp_user_id === p.user_id && <MvpTag />}
              <span className="row-end">
                {match.status === 'voting' ? (
                  <VoteTag match={match} userId={p.user_id} />
                ) : (
                  <span className="mono muted small">{points(p.points_after ?? player?.points ?? 0)}</span>
                )}
              </span>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function NextStep({ camp, match }: { camp: Camp; match: Match }) {
  const { run, busy } = useActions();
  const [winner, setWinner] = useState<Side | null>(null);
  const current = stage(match);
  const missing = match.players.length - match.votes.length;

  if (match.status === 'completed') {
    const latest = camp.recent_matches[0];
    return (
      <section className="card" aria-labelledby="result-title">
        <h2 id="result-title" className="small-title">
          Resultado
        </h2>
        {match.winner_side && (
          <p>
            <span className={`tag tag-${match.winner_side}`}>{SIDE_LABEL[match.winner_side]}</span> venceu. +1 ponto para
            cada jogador do time.
          </p>
        )}
        <p className="small">
          {match.mvp_user_id ? (
            <>
              <MvpTag /> {playerName(camp, match.mvp_user_id)}
            </>
          ) : (
            <span className="muted">Sem MVP.</span>
          )}
        </p>
        {latest?.id === match.id ? (
          <Corrections camp={camp} match={latest} />
        ) : (
          <p className="muted small">Só a última partida pode ser corrigida, e só enquanto os jogadores dela estão na fila.</p>
        )}
      </section>
    );
  }

  if (current === 'live') {
    return (
      <section className="card" aria-labelledby="next-title">
        <h2 id="next-title" className="small-title">
          Próximo passo
        </h2>
        <div className="actions two">
          <button type="button" className="button button-hk" disabled={busy} onClick={() => setWinner('hiddenking')}>
            Hiddenking venceu
          </button>
          <button type="button" className="button button-am" disabled={busy} onClick={() => setWinner('archmother')}>
            Archmother venceu
          </button>
        </div>
        <span className="muted small">
          Os vencedores ganham 1 ponto e todos voltam para a fila e para o canal Queue. Você escolhe o MVP em seguida.
        </span>
        <WinnerDialog camp={camp} match={match} side={winner} onClose={() => setWinner(null)} />
      </section>
    );
  }

  if (current === 'draft') {
    return (
      <section className="card" aria-labelledby="next-title">
        <h2 id="next-title" className="small-title">
          Próximo passo
        </h2>
        {draftStarted(match) ? (
          <>
            <button
              type="button"
              className="button button-primary"
              disabled={busy}
              onClick={() => run({ type: 'start_match', match_id: match.id })}
            >
              Iniciar partida
            </button>
            <span className="muted small">Quando o draft terminar. Se o lobby automático estiver ligado, o bot posta o código da party.</span>
          </>
        ) : (
          <>
            <button
              type="button"
              className="button button-primary"
              disabled={busy}
              onClick={() => run({ type: 'start_draft', match_id: match.id })}
            >
              {camp.draft_tool ? 'Iniciar draft' : 'Iniciar draft manual'}
            </button>
            <span className="muted small">
              {camp.draft_tool
                ? camp.dm_captains
                  ? 'Cria o draft no DeadChaps, manda o link aos capitães por DM e move todos para os canais de voz dos times.'
                  : 'Cria o draft no DeadChaps e move todos para os canais de voz dos times. Os capitães pegam o link no botão Link do capitão da mensagem da partida.'
                : 'A ferramenta de draft está desligada: o bot move todos para os canais de voz dos times e avisa no canal. Você manda o link do draft para os capitães.'}
            </span>
          </>
        )}
      </section>
    );
  }

  return (
    <>
      <section className="card" aria-labelledby="next-title">
        <h2 id="next-title" className="small-title">
          Próximo passo
        </h2>
        <button type="button" className="button button-primary" disabled aria-describedby="draft-help">
          Iniciar draft
        </button>
        <span id="draft-help" className="muted small">
          {camp.draft_tool
            ? `Libera quando a votação fechar. O draft é criado no DeadChaps, os capitães ${
                camp.dm_captains ? 'recebem o link por DM' : 'pegam o link no botão Link do capitão'
              } e todos vão para os canais de voz dos times.`
            : 'Libera quando a votação fechar. O draft é manual: o bot move todos para os canais dos times e você manda o link.'}
        </span>
        <button
          type="button"
          className="button"
          disabled={busy || missing === 0}
          onClick={() =>
            run(
              { type: 'remind_voters', match_id: match.id },
              { success: `Lembrete enviado no Discord para ${missing === 1 ? '1 jogador' : `${missing} jogadores`}.` },
            )
          }
        >
          Lembrar quem falta votar ({missing})
        </button>
        <span className="muted small">Menciona no Discord só quem ainda não votou.</span>
      </section>
      <section className="card" aria-labelledby="close-title">
        <h2 id="close-title" className="small-title">
          Fechar a votação agora
        </h2>
        <p className="small text-2">
          Para quando alguém não pode votar ou o resultado já está claro: mantenha os times ou sorteie de novo. Depois
          disso, o draft libera.
        </p>
        <div className="actions two">
          <button
            type="button"
            className="button"
            disabled={busy}
            onClick={() => run({ type: 'close_vote', match_id: match.id }, { confirm: confirmCloseVote(match) })}
          >
            Manter times
          </button>
          <button
            type="button"
            className="button"
            disabled={busy}
            onClick={() => run({ type: 'refresh_teams', match_id: match.id }, { confirm: confirmRefreshTeams(match) })}
          >
            Sortear de novo
          </button>
        </div>
      </section>
    </>
  );
}

function MatchDetail({ camp, match }: { camp: Camp; match: Match }) {
  const voting = match.status === 'voting';
  return (
    <div className="stack detail">
      <nav aria-label="Caminho" className="breadcrumb small">
        <a href={routeHref('matches')}>Partidas</a> <span aria-hidden="true">/</span> {detailName(match)}
      </nav>
      <div className="title-row title-row-wrap">
        <h1 className="page-title">{detailName(match)}</h1>
        <span className="mono muted">{match.id}</span>
        <StagePill match={match} />
      </div>
      <div className="stepper-wide">
        <Stepper match={match} />
      </div>
      {voting && <VoteTiles camp={camp} match={match} />}
      <div className="split">
        <section className="split-main card" aria-labelledby="teams-title">
          <h2 id="teams-title">{voting ? 'Times e votos' : 'Times'}</h2>
          <div className="teams">
            {SIDES.map((side) => (
              <TeamVotes key={side} camp={camp} match={match} side={side} />
            ))}
          </div>
          <p className="muted small">
            {voting && !match.refresh_used
              ? `A votação fecha quando ${camp.refresh_votes} jogadores de um mesmo time pedem refresh, quando todos votam, ou quando o refresh não tem mais como passar.`
              : voteSummary(camp, match)}
          </p>
        </section>
        <aside className="split-aside" aria-label="Ações da partida">
          <NextStep camp={camp} match={match} />
          {match.draft && <DraftLinks camp={camp} match={match} draft={match.draft} />}
          {match.draft_manual && <ManualDraftNote camp={camp} match={match} />}
          {match.party_code && (
            <div className="inset link-row">
              <span className="eyebrow">Código da party</span>
              <span className="mono">{match.party_code}</span>
              <CopyButton text={match.party_code} label="Copiar código da party" />
            </div>
          )}
          <section className="card card-dashed" aria-labelledby="discord-title">
            <h2 id="discord-title" className="small-title">
              No Discord
            </h2>
            <p className="small text-2">
              {voting && !match.refresh_used
                ? 'A mensagem da partida mostra os times e os botões Refresh e Manter. Os números aqui atualizam a cada voto.'
                : 'A mensagem da partida mostra os times e a etapa atual; o resultado fica nela quando a partida termina.'}
            </p>
            <DiscordLink url={match.discord_url} label="Abrir a mensagem no Discord" />
          </section>
        </aside>
      </div>
    </div>
  );
}
