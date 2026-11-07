// Custom CSS for the DeadChaps draft overlay (the draft link plus /obs) as an OBS browser source.

/** OBS's default browser-source CSS: keeps the overlay transparent. */
const OBS_DEFAULT_CSS = 'body { background-color: rgba(0, 0, 0, 0); margin: 0px auto; overflow: hidden; }';

/**
 * Each hero portrait's width / height and where its face is across it (0 left, 1 right), from DeadChaps' own
 * draft page (its draftImageHelper). The overlay crops every portrait around its centre, cutting most faces off.
 */
const PORTRAIT_FACES: Record<string, [ratio: number, faceX: number]> = {
  Abrams: [1.1378, 0.4759],
  Apollo: [1.3708, 0.5467],
  Baba: [1.1597, 0.5523],
  Bebop: [1.1327, 0.7154],
  Billy: [1.3603, 0.6335],
  Calico: [1.2225, 0.6053],
  Celeste: [1.1327, 0.6449],
  DeadmanDanny: [0.9808, 0.37],
  Drifter: [1.7778, 0.6933],
  Dynamo: [1.0271, 0.66],
  Graves: [1.1716, 0.3299],
  GreyTalon: [1.2068, 0.4149],
  Haze: [1.047, 0.5337],
  Holliday: [1.0859, 0.4046],
  Infernus: [1.7778, 0.7089],
  Ivy: [1.1996, 0.5262],
  Kelvin: [1.2367, 0.6136],
  LadyGeist: [1.2736, 0.4651],
  Lash: [1.7778, 0.5757],
  McGinnis: [1.5086, 0.4215],
  Mina: [1, 0.3896],
  Mirage: [1.7778, 0.6497],
  MoAndKrill: [1.1716, 0.584],
  NurseHarrow: [0.9394, 0.6048],
  Paige: [1.1167, 0.514],
  Paradox: [1.183, 0.6682],
  Pocket: [1.2686, 0.5938],
  RatKing: [1.1378, 0.45],
  Rem: [1.1597, 0.4422],
  Seven: [1.1663, 0.5111],
  Shiv: [1.2559, 0.6518],
  Silver: [1.0644, 0.6836],
  Sinclair: [1.0847, 0.5346],
  Solomon: [1.1403, 0.547],
  TheDoorman: [1.1327, 0.4172],
  Venator: [1.0838, 0.6],
  Victor: [1.0999, 0.5212],
  Vindicta: [1.1327, 0.6558],
  Violet: [1.0407, 0.8],
  Viscous: [1.2156, 0.5591],
  Vyper: [1.4447, 0.5387],
  Warden: [1.0503, 0.5804],
  Wraith: [1.1327, 0.4532],
  Yamato: [1.0802, 0.4033],
};

/** The overlay's hero slots are 125×250. */
const SLOT_RATIO = 0.5;

/**
 * Where to anchor a portrait so its face sits mid-slot. With `object-fit: cover` a portrait fills the slot's
 * height, and `object-position: p%` lines up the portrait's p% point with the slot's.
 */
function facePosition(ratio: number, faceX: number): string {
  const p = (ratio * faceX - SLOT_RATIO / 2) / (ratio - SLOT_RATIO);
  return `${(Math.min(1, Math.max(0, p)) * 100).toFixed(1)}%`;
}

/** For the browser source's Custom CSS: OBS's default, then each hero centred on their face. */
export const OBS_OVERLAY_CSS = [
  OBS_DEFAULT_CSS,
  '/* DeadChaps draft overlay: centre each hero on their face. */',
  ...Object.entries(PORTRAIT_FACES).map(
    ([hero, [ratio, faceX]]) =>
      `.obs-hero-slot img[src$="/${hero}.webp"] { object-position: ${facePosition(ratio, faceX)} 50%; }`,
  ),
].join('\n');
