import { useEffect, useMemo, useState } from 'react';

import { ActionsContext, type Actions } from './actions';
import { Unauthorized, api, type Camp, type CampSummary, type Me } from './api';
import { isActive } from './camp';
import { FullPage } from './components/common';
import { Header } from './components/Header';
import { LiveView } from './components/LiveView';
import { NavRail } from './components/NavRail';
import { remember, remembered, usePolling } from './usePolling';

const loadMe = () => api.me();

function Login() {
  return (
    <FullPage title="Refresher Admin">
      <p className="muted">Entre com a sua conta do Discord. Quem tem Gerenciar Servidor no servidor do camp é admin.</p>
      <a className="button button-primary button-wide" href="/auth/login">
        Entrar com Discord
      </a>
    </FullPage>
  );
}

/** The remembered choice if it's still there, else the best default. */
function pick<T extends { id: string }>(items: T[], stored: string | null, fallback: (items: T[]) => T | undefined) {
  return items.find((i) => i.id === stored) ?? fallback(items);
}

function Admin({ me }: { me: Me }) {
  const [guildId, setGuildId] = useState(() => pick(me.guilds, remembered('guild'), (g) => g[0])?.id ?? null);
  const camps = usePolling<CampSummary[]>(guildId, api.camps, 30_000);
  const [campId, setCampId] = useState<string | null>(null);

  useEffect(() => {
    if (!camps.data) return;
    setCampId((current) => {
      const stored = current ?? remembered('camp');
      return pick(camps.data!, stored, (list) => list.find((c) => isActive(c.status)) ?? list[0])?.id ?? null;
    });
  }, [camps.data]);

  const camp = usePolling<Camp>(campId, api.camp, 4_000);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const replaceCamp = camp.replace;

  const actions = useMemo<Actions>(
    () => ({
      busy,
      run: (action, confirm) => {
        if (!campId || busy) return;
        if (confirm && !window.confirm(confirm)) return;
        setBusy(true);
        setNotice(null);
        api
          .act(campId, action)
          .then(replaceCamp)
          .catch((error: Error) => setNotice(error.message))
          .finally(() => setBusy(false));
      },
    }),
    [busy, campId, replaceCamp],
  );

  if (camp.error instanceof Unauthorized || camps.error instanceof Unauthorized) return <Login />;

  const chooseGuild = (id: string) => {
    remember('guild', id);
    setGuildId(id);
    setCampId(null);
  };
  const chooseCamp = (id: string) => {
    remember('camp', id);
    setCampId(id);
  };

  let content;
  if (!guildId) {
    content = (
      <section className="card empty-card">
        <h2>Nenhum servidor</h2>
        <p className="muted small">Você não gerencia nenhum servidor onde o bot está.</p>
      </section>
    );
  } else if (camps.data && camps.data.length === 0) {
    content = (
      <section className="card empty-card">
        <h2>Nenhum camp neste servidor</h2>
        <p className="muted small">
          Crie um com <span className="mono">/camp iniciar</span> no Discord. Criar camps pelo painel chega na próxima etapa.
        </p>
      </section>
    );
  } else if (camp.data) {
    content = <LiveView camp={camp.data} />;
  } else {
    content = <p className="muted">{camp.error?.message ?? camps.error?.message ?? 'Carregando…'}</p>;
  }

  return (
    <ActionsContext.Provider value={actions}>
      <div className="app">
        <Header
          me={me}
          guildId={guildId ?? ''}
          onGuild={chooseGuild}
          camps={camps.data ?? []}
          campId={campId}
          onCamp={chooseCamp}
          camp={camp.data}
        />
        {camp.data && camp.error && (
          <p className="banner" role="status">
            Sem conexão com o servidor: {camp.error.message}. Mostrando os últimos dados.
          </p>
        )}
        <div className="status-area" role="status">
          {busy && <p className="working">Enviando ao bot… o Discord é atualizado junto.</p>}
        </div>
        {notice && (
          <div className="notice" role="alert">
            <span>{notice}</span>
            <button type="button" className="icon-button" aria-label="Fechar aviso" onClick={() => setNotice(null)}>
              ×
            </button>
          </div>
        )}
        <div className="body">
          <NavRail />
          <main className="main">{content}</main>
        </div>
      </div>
    </ActionsContext.Provider>
  );
}

export default function App() {
  const me = usePolling<Me>('me', loadMe, 60_000);
  if (me.error instanceof Unauthorized) return <Login />;
  if (!me.data) {
    return (
      <FullPage title={me.error ? 'Não foi possível carregar' : 'Carregando…'}>
        {me.error && <p className="muted">{me.error.message}</p>}
      </FullPage>
    );
  }
  return <Admin me={me.data} />;
}
