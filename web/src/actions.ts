import { createContext, useContext } from 'react';

import type { Camp, CampAction, Match } from './api';
import { matchName } from './camp';

/** A question asked before an action, shown in the page's confirm dialog. */
export interface Confirm {
  title: string;
  /** What will happen. */
  message: string;
  /** A second, quieter line: what stays, what can be undone. */
  note?: string;
  /** The confirm button: the action itself, e.g. "Encerrar camp". */
  action: string;
  /** `danger` for what can't be undone; a team color for its winner. Default `primary`. */
  tone?: 'primary' | 'danger' | 'hiddenking' | 'archmother';
}

export interface RunOptions {
  /** Ask first. */
  confirm?: Confirm;
  /** Act on this camp instead of the one on screen. */
  campId?: string;
  /** Shown for a few seconds after the action works, when the page itself wouldn't show it. */
  success?: string;
  /** Called with the camp after the action. */
  then?: (camp: Camp) => void;
}

export interface Actions {
  /** Runs an action through the bot. One at a time. */
  run: (action: CampAction, options?: RunOptions) => void;
  /** An action is running; buttons wait for it. */
  busy: boolean;
}

export const ActionsContext = createContext<Actions>({ run: () => {}, busy: true });

export function useActions(): Actions {
  return useContext(ActionsContext);
}

/** Before an admin ends a refresh vote early. */
export function confirmCloseVote(match: Match): Confirm {
  const missing = match.players.length - match.votes.length;
  return {
    title: `Encerrar a votação da ${matchName(match)}?`,
    message: `Os times ficam como estão e o draft libera. ${missing === 1 ? 'Falta 1 voto.' : `Faltam ${missing} votos.`}`,
    note: 'Os botões de voto somem da mensagem da partida no Discord.',
    action: 'Encerrar e manter times',
  };
}

/** Before an admin ends a refresh vote early by re-sorting the teams. */
export function confirmRefreshTeams(match: Match): Confirm {
  return {
    title: `Sortear os times da ${matchName(match)} de novo?`,
    message: 'A votação acaba e os mesmos jogadores são sorteados em times novos, com os S rank separados e capitães escolhidos de novo. O draft libera em seguida.',
    note: 'Os votos dados até agora são descartados, e a partida não tem outro refresh.',
    action: 'Sortear de novo',
  };
}

/** Why "Criar partida" is off, in the bot's words. */
export function createMatchBlocked(camp: Camp): string | undefined {
  if (camp.can_create_match) return undefined;
  if (camp.status !== 'running') return 'O camp não está em andamento.';
  if (camp.has_final) return 'A partida final já foi criada.';
  if (camp.open_matches.length >= 2) return 'As duas vagas de partida estão ocupadas.';
  return `Precisa de ${camp.match_size} jogadores na fila.`;
}

/** Why "Partida final" is off. */
export function createFinalBlocked(camp: Camp): string | undefined {
  if (camp.can_create_final) return undefined;
  if (camp.has_final) return 'A partida final já foi criada.';
  if (camp.open_matches.length > 0) return 'Termine as partidas abertas antes da final.';
  if (camp.match_count < 1) return 'Jogue pelo menos uma partida antes da final.';
  return 'Ainda não dá para criar a final.';
}
