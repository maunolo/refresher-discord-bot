import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { overlayPath } from './data';
import { RankingOverlay } from './RankingOverlay';
import { VoteOverlay } from './VoteOverlay';
import './overlay.css';

// `/overlay/{secret}/votacao` (optionally `?partida=1` for one match's vote) and
// `/overlay/{secret}/ranking` (optionally `?top=15`, 10 by default).
const path = overlayPath();
const query = new URLSearchParams(window.location.search);
const slot = Number(query.get('partida')) || null;
const top = Math.min(Math.max(Number(query.get('top')) || 10, 1), 30);

function Overlay() {
  if (path?.overlay === 'votacao') return <VoteOverlay token={path.token} slot={slot} />;
  if (path?.overlay === 'ranking') return <RankingOverlay token={path.token} top={top} />;
  return null;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Overlay />
  </StrictMode>,
);
