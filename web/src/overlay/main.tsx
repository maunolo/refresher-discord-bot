import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { CamerasOverlay } from './CamerasOverlay';
import { overlayPath } from './data';
import { RankingOverlay } from './RankingOverlay';
import { VoteOverlay } from './VoteOverlay';
import './overlay.css';

// `/overlay/{secret}/votacao` (optionally `?partida=1` for one match's vote),
// `/overlay/{secret}/ranking` (optionally `?top=15`, 10 by default), and
// `/overlay/{secret}/cameras1` and `cameras2`, one per stream (optionally `?debug=1` or `?demo=1`).
const path = overlayPath();
const query = new URLSearchParams(window.location.search);
const slot = Number(query.get('partida')) || null;
const top = Math.min(Math.max(Number(query.get('top')) || 10, 1), 30);
const flag = (name: string) => query.has(name) && query.get(name) !== '0';
const cameraStream = /^cameras([12])$/.exec(path?.overlay ?? '');

function Overlay() {
  if (path?.overlay === 'votacao') return <VoteOverlay token={path.token} slot={slot} />;
  if (path?.overlay === 'ranking') return <RankingOverlay token={path.token} top={top} />;
  if (path && cameraStream) {
    return <CamerasOverlay token={path.token} stream={Number(cameraStream[1])} debug={flag('debug')} demo={flag('demo')} />;
  }
  return null;
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Overlay />
  </StrictMode>,
);
