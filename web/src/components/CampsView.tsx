import { useEffect, useState, type FormEvent } from 'react';

import { useActions } from '../actions';
import { api, type Camp, type CampSizeChoice, type CampSummary, type Channel } from '../api';
import { campStatusLabel, formatLabel, isActive, playerName, points, ranking } from '../camp';
import { DiscordLink } from './common';

const SIZES: { value: CampSizeChoice; title: string; detail: string }[] = [
  { value: '24', title: '24 jogadores', detail: 'Duas partidas 6v6 ao mesmo tempo' },
  { value: '30', title: '30 jogadores', detail: 'Duas partidas 6v6, seis esperando na fila' },
  { value: '4-1v1', title: 'Teste 4 · 1v1', detail: 'Duas partidas 1v1' },
  { value: '4-2v2', title: 'Teste 4 · 2v2', detail: 'Uma partida 2v2' },
];

function sizeLabel(camp: CampSummary): string {
  return camp.target_size === 4 ? `Teste 4 · ${formatLabel(camp.match_size)}` : `${camp.target_size} · ${formatLabel(camp.match_size)}`;
}

function createdOn(seconds: number): string {
  return new Date(seconds * 1000).toLocaleDateString('pt-BR', { day: '2-digit', month: 'short' });
}

function StatusPill({ status }: { status: string }) {
  const tone = status === 'running' ? 'pill-running' : status === 'signup' ? 'pill-neutral' : 'pill-muted';
  return <span className={`pill ${tone}`}>{campStatusLabel(status)}</span>;
}

function NewCampForm({
  guildId,
  channels,
  onCreated,
  onClose,
}: {
  guildId: string;
  channels: Channel[];
  onCreated: (camp: Camp) => void;
  onClose: () => void;
}) {
  const [channelId, setChannelId] = useState(channels[0]?.id ?? '');
  const [size, setSize] = useState<CampSizeChoice>('24');
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!channelId && channels[0]) setChannelId(channels[0].id);
  }, [channels, channelId]);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    setSending(true);
    setError(null);
    api
      .createCamp(guildId, channelId, size)
      .then(onCreated)
      .catch((err: Error) => setError(err.message))
      .finally(() => setSending(false));
  };

  return (
    <form className="card" aria-labelledby="new-camp-title" onSubmit={submit}>
      <h2 id="new-camp-title">Novo camp</h2>
      <div className="field">
        <label htmlFor="camp-channel">Canal de texto</label>
        <select id="camp-channel" className="input" value={channelId} onChange={(e) => setChannelId(e.target.value)} required>
          {channels.map((c) => (
            <option key={c.id} value={c.id}>
              #{c.name}
            </option>
          ))}
        </select>
        <span className="muted small">O bot posta a fila e as partidas aqui.</span>
      </div>
      <fieldset className="field">
        <legend>Tamanho do camp</legend>
        <div className="choices">
          {SIZES.map((s) => (
            <label key={s.value} className={s.value === size ? 'choice choice-on' : 'choice'}>
              <input type="radio" name="camp-size" value={s.value} checked={s.value === size} onChange={() => setSize(s.value)} />
              <span>
                <strong>{s.title}</strong>
                <span className="muted small">{s.detail}</span>
              </span>
            </label>
          ))}
        </div>
      </fieldset>
      {error && <p className="error small">{error}</p>}
      <div className="form-actions">
        <button type="submit" className="button button-primary" disabled={sending || !channelId}>
          {sending ? 'Criando…' : 'Criar camp'}
        </button>
        <button type="button" className="button button-ghost" onClick={onClose}>
          Fechar
        </button>
      </div>
    </form>
  );
}

function FinalRanking({ campId }: { campId: string }) {
  const [camp, setCamp] = useState<Camp | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setCamp(null);
    api.camp(campId).then(setCamp).catch((e: Error) => setError(e.message));
  }, [campId]);

  return (
    <aside className="card split-aside-card" aria-labelledby="final-title">
      <div>
        <h2 id="final-title">Ranking final</h2>
        <span className="muted small">
          Camp <span className="mono">{campId}</span>
          {camp && ` · ${camp.match_count} partidas`}
        </span>
      </div>
      {error && <p className="error small">{error}</p>}
      {!camp && !error && <p className="muted">Carregando…</p>}
      {camp && (
        <ol className="rows">
          {ranking(camp.players)
            .slice(0, 12)
            .map(({ player, place }) => (
              <li key={player.user_id} className="row row-tall">
                <span className={`medal medal-${Math.min(place, 4)}`}>{place}</span>
                <span className="ellipsis">{playerName(camp, player.user_id)}</span>
                <span className="row-end mono small">{points(player.points)}</span>
              </li>
            ))}
        </ol>
      )}
    </aside>
  );
}

interface CampsViewProps {
  guildId: string;
  camps: CampSummary[];
  onOpen: (campId: string) => void;
  onCreated: (camp: Camp) => void;
}

export function CampsView({ guildId, camps, onOpen, onCreated }: CampsViewProps) {
  const { run, busy } = useActions();
  const [creating, setCreating] = useState(false);
  const [rankingOf, setRankingOf] = useState<string | null>(null);
  const [channels, setChannels] = useState<Channel[]>([]);

  useEffect(() => {
    api.channels(guildId).then(setChannels).catch(() => setChannels([]));
  }, [guildId]);

  const active = camps.find((c) => isActive(c.status));
  const channelName = (id: string) => channels.find((c) => c.id === id)?.name;

  return (
    <div className="split">
      <section className="split-main stack" aria-labelledby="camps-title">
        <div className="card-head">
          <h1 id="camps-title" className="page-title">
            Camps
          </h1>
          <div className="inline-actions">
            {active && <span className="muted small">Um camp ativo por servidor: cancele ou encerre o {active.id} para criar outro.</span>}
            <button type="button" className="button button-primary" disabled={!!active || creating} onClick={() => setCreating(true)}>
              Novo camp
            </button>
          </div>
        </div>
        {creating && (
          <NewCampForm
            guildId={guildId}
            channels={channels}
            onClose={() => setCreating(false)}
            onCreated={(camp) => {
              setCreating(false);
              onCreated(camp);
            }}
          />
        )}
        {camps.length === 0 ? (
          <section className="card empty-card">
            <h2>Nenhum camp neste servidor ainda</h2>
            <p className="muted small">Clique em Novo camp para abrir a inscrição.</p>
          </section>
        ) : (
          <div className="card table-scroll">
            <table className="table">
              <thead>
                <tr>
                  <th scope="col">Camp</th>
                  <th scope="col">Status</th>
                  <th scope="col">Tamanho</th>
                  <th scope="col">Partidas</th>
                  <th scope="col">Criado</th>
                  <th scope="col">Canal</th>
                  <th scope="col" className="align-end">
                    <span className="visually-hidden">Ações</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {camps.map((c) => (
                  <tr key={c.id}>
                    <td className="mono">{c.id}</td>
                    <td>
                      <StatusPill status={c.status} />
                    </td>
                    <td>{sizeLabel(c)}</td>
                    <td>{c.match_count}</td>
                    <td className="muted">{createdOn(c.created_at)}</td>
                    <td>
                      <DiscordLink url={c.discord_url} label={channelName(c.channel_id) ? `#${channelName(c.channel_id)}` : 'Discord'} />
                    </td>
                    <td className="align-end">
                      <div className="inline-actions">
                        {isActive(c.status) && (
                          <>
                            <button type="button" className="button button-small button-primary" onClick={() => onOpen(c.id)}>
                              Abrir
                            </button>
                            <button
                              type="button"
                              className="button button-small button-ghost"
                              disabled={busy}
                              onClick={() =>
                                run(
                                  { type: 'cancel_camp' },
                                  {
                                    campId: c.id,
                                    confirm: {
                                      title: `Cancelar o camp ${c.id}?`,
                                      message: 'O camp para agora. Elenco, fila e pontos ficam guardados.',
                                      note: 'Dá para retomar depois, aqui em Camps.',
                                      action: 'Cancelar camp',
                                      tone: 'danger',
                                    },
                                  },
                                )
                              }
                            >
                              Cancelar
                            </button>
                          </>
                        )}
                        {c.status === 'running' && (
                          <button
                            type="button"
                            className="button button-small button-danger"
                            disabled={busy || c.match_count < 1}
                            title={c.match_count < 1 ? 'Jogue pelo menos uma partida antes de encerrar.' : undefined}
                            onClick={() =>
                              run(
                                { type: 'end_camp' },
                                {
                                  campId: c.id,
                                  confirm: {
                                    title: `Encerrar o camp ${c.id}?`,
                                    message: 'O ranking final é publicado no Discord e o camp termina.',
                                    note: 'Não dá para desfazer.',
                                    action: 'Encerrar camp',
                                    tone: 'danger',
                                  },
                                  then: () => setRankingOf(c.id),
                                },
                              )
                            }
                          >
                            Encerrar
                          </button>
                        )}
                        {c.status.startsWith('cancelled') && (
                          <button
                            type="button"
                            className="button button-small"
                            disabled={busy || !!active}
                            title={active ? 'Só dá para retomar quando não houver outro camp ativo.' : undefined}
                            onClick={() => run({ type: 'resume_camp' }, { campId: c.id, then: (camp) => onOpen(camp.id) })}
                          >
                            Retomar
                          </button>
                        )}
                        {c.status === 'ended' && (
                          <button
                            type="button"
                            className="button button-small"
                            aria-pressed={rankingOf === c.id}
                            onClick={() => setRankingOf(c.id)}
                          >
                            Ver ranking
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
        <p className="muted small">
          Camps cancelados guardam elenco, fila e pontos e podem ser retomados. Encerrar publica o ranking final no Discord e
          não tem volta.
        </p>
      </section>
      {rankingOf && (
        <div className="split-aside">
          <FinalRanking campId={rankingOf} />
        </div>
      )}
    </div>
  );
}
