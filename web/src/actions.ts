import { createContext, useContext } from 'react';

import type { Camp, CampAction } from './api';

export interface Actions {
  /** Runs an action, after `confirm` if given. One at a time. */
  run: (action: CampAction, confirm?: string) => void;
  /** An action is running; buttons wait for it. */
  busy: boolean;
}

export const ActionsContext = createContext<Actions>({ run: () => {}, busy: true });

export function useActions(): Actions {
  return useContext(ActionsContext);
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
