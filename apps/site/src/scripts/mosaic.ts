// Mosaico interativo do hero (docs/08-site.md). Clique dispara uma onda, o mouse acende os
// ladrilhos por perto e, ocioso, alguns cintilam. Só anima quando há algo para mostrar e a
// seção está visível; com movimento reduzido, o clique vira um brilho local sem onda.
// Parâmetros calibrados no protótipo (prototypes/mosaico.html, painel "Ajustes").

const PARAMS = {
  tile: 36, // px (no celular é reduzido para ~28)
  gap: 2, // px entre ladrilhos
  speed: 0.9, // px/ms: velocidade da frente da onda
  pulse: 650, // ms: duração do pulso em cada ladrilho
  falloff: 650, // px: decaimento exp(-d / falloff)
  glow: 0.6, // 0..1: intensidade do brilho aditivo
  hoverRadius: 120, // px
  twinkle: true, // cintilar ocioso
};
const MAX_WAVES = 14;
const ATTACK = 0.12; // fração do pulso gasta subindo até o pico
const FLASH_MS = 420; // clarão no ponto do clique
const TWINKLE_MS = 2600;
const TWINKLE_DENSITY = 0.018; // fração de ladrilhos cintilando ao mesmo tempo
const IDLE_FPS = 24; // só o cintilar ativo: renderiza mais devagar
const GLOW_MS = 450; // brilho local no modo de movimento reduzido
const NO_WAVE = "a, button, input, select, textarea, label, [data-no-wave]";

// Rampa de cor: base → #1D4ED8 → #3B82F6 → #93C5FD
type Rgb = [number, number, number];
const STOPS: [number, Rgb][] = [
  [0, [11, 17, 34]],
  [0.45, [29, 78, 216]],
  [0.75, [59, 130, 246]],
  [1, [147, 197, 253]],
];
const LUT_SIZE = 64;
const LUT = Array.from({ length: LUT_SIZE }, (_, i) => {
  const t = i / (LUT_SIZE - 1);
  let j = 0;
  while (j < STOPS.length - 2 && t > STOPS[j + 1][0]) j++;
  const [t0, c0] = STOPS[j];
  const [t1, c1] = STOPS[j + 1];
  const k = Math.min(1, Math.max(0, (t - t0) / (t1 - t0)));
  const mix = (a: number, b: number) => Math.round(a + (b - a) * k);
  return `rgb(${mix(c0[0], c1[0])},${mix(c0[1], c1[1])},${mix(c0[2], c1[2])})`;
});

interface Wave {
  x: number;
  y: number;
  t0: number;
}

type Ctx = CanvasRenderingContext2D;
const roundRect = (c: Ctx, x: number, y: number, w: number, h: number, r: number) =>
  c.roundRect ? c.roundRect(x, y, w, h, r) : c.rect(x, y, w, h);

/** Liga o mosaico ao `canvas` dentro de `hero`; `onFirstWave` roda no primeiro clique. */
export function mountMosaic(hero: HTMLElement, canvas: HTMLCanvasElement, onFirstWave: () => void) {
  const ctx = canvas.getContext("2d", { alpha: false });
  const base = document.createElement("canvas");
  const bctx = base.getContext("2d", { alpha: false });
  if (!ctx || !bctx) return;

  let W = 0;
  let H = 0;
  let dpr = 1;
  let tile = PARAMS.tile;
  let n = 0;
  let tx = new Float32Array(0);
  let ty = new Float32Array(0);
  let cx = new Float32Array(0);
  let cy = new Float32Array(0);
  let twStart = new Float64Array(0);
  let twPeak = new Float32Array(0);
  const buckets: number[][] = Array.from({ length: LUT_SIZE }, () => []);
  const hot: number[] = [];
  const waves: Wave[] = [];
  const glows: Wave[] = [];
  const hover = { x: 0, y: 0, active: false };

  let raf = 0;
  let dirty = true;
  let lastRender = 0;
  let lastTwinkle = 0;
  let twinkleAcc = 0;
  let pageVisible = !document.hidden;
  let onScreen = true;
  let waved = false;
  const reduceMq = window.matchMedia("(prefers-reduced-motion: reduce)");
  const reduced = () => reduceMq.matches;

  // ── Grade ──────────────────────────────────────────────────────────
  function layout() {
    const rect = hero.getBoundingClientRect();
    W = Math.max(1, Math.round(rect.width));
    H = Math.max(1, Math.round(rect.height));
    dpr = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(W * dpr);
    canvas.height = Math.round(H * dpr);
    base.width = canvas.width;
    base.height = canvas.height;

    tile = Math.max(12, Math.round(PARAMS.tile * (window.innerWidth < 640 ? 28 / 36 : 1)));
    const step = tile + PARAMS.gap;
    const cols = Math.ceil(W / step) + 1;
    const rows = Math.ceil(H / step) + 1;
    const ox = Math.round((W - (cols * step - PARAMS.gap)) / 2);
    const oy = Math.round((H - (rows * step - PARAMS.gap)) / 2);

    n = cols * rows;
    tx = new Float32Array(n);
    ty = new Float32Array(n);
    cx = new Float32Array(n);
    cy = new Float32Array(n);
    twStart = new Float64Array(n).fill(-1e9);
    twPeak = new Float32Array(n);
    for (let r = 0; r < rows; r++) {
      for (let c = 0; c < cols; c++) {
        const i = r * cols + c;
        tx[i] = ox + c * step;
        ty[i] = oy + r * step;
        cx[i] = tx[i] + tile / 2;
        cy[i] = ty[i] + tile / 2;
      }
    }
    drawBase();
    requestRender();
  }

  function drawBase() {
    const rad = Math.max(2, tile * 0.12);
    bctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
    bctx!.fillStyle = "#060A14";
    bctx!.fillRect(0, 0, W, H);
    bctx!.fillStyle = "#131B31";
    bctx!.beginPath();
    for (let i = 0; i < n; i++) roundRect(bctx!, tx[i], ty[i], tile, tile, rad);
    bctx!.fill();
    bctx!.fillStyle = "#0B1122";
    bctx!.beginPath();
    for (let i = 0; i < n; i++) roundRect(bctx!, tx[i] + 1, ty[i] + 1, tile - 2, tile - 2, rad - 1);
    bctx!.fill();
  }

  // ── Renderização ───────────────────────────────────────────────────
  function spawnTwinkles(now: number) {
    const dt = lastTwinkle ? Math.min(100, now - lastTwinkle) : 16;
    lastTwinkle = now;
    twinkleAcc += ((n * TWINKLE_DENSITY) / TWINKLE_MS) * dt;
    while (twinkleAcc >= 1) {
      twinkleAcc -= 1;
      const i = (Math.random() * n) | 0;
      if (now - twStart[i] > TWINKLE_MS) {
        twStart[i] = now;
        twPeak[i] = 0.08 + Math.random() * 0.16;
      }
    }
  }

  function render(now: number) {
    const c = ctx!;
    const rm = reduced();
    const { speed, pulse, falloff, glow, hoverRadius } = PARAMS;
    const band = speed * pulse;
    const invFall = 1 / falloff;

    // remove ondas que já saíram da tela ou ficaram fracas demais
    const maxD = Math.min(Math.hypot(W, H), falloff * 4.6);
    for (let k = waves.length - 1; k >= 0; k--) {
      if (now - waves[k].t0 > maxD / speed + pulse) waves.splice(k, 1);
    }
    for (let k = glows.length - 1; k >= 0; k--) {
      if (now - glows[k].t0 > GLOW_MS) glows.splice(k, 1);
    }

    const doTwinkle = PARAMS.twinkle && !rm;
    if (doTwinkle) spawnTwinkles(now);
    const doHover = hover.active && !rm && hoverRadius > 0;
    const hr2 = hoverRadius * hoverRadius;

    for (const b of buckets) b.length = 0;
    hot.length = 0;

    for (let i = 0; i < n; i++) {
      let keep = 1; // produto de (1 - energia): várias fontes se somam sem estourar

      if (doTwinkle) {
        const age = now - twStart[i];
        if (age >= 0 && age < TWINKLE_MS) keep *= 1 - twPeak[i] * Math.sin((Math.PI * age) / TWINKLE_MS);
      }
      if (doHover) {
        const dx = cx[i] - hover.x;
        const dy = cy[i] - hover.y;
        const d2 = dx * dx + dy * dy;
        if (d2 < hr2) {
          const f = 1 - Math.sqrt(d2) / hoverRadius;
          keep *= 1 - 0.24 * f * f;
        }
      }
      for (const w of waves) {
        const dx = cx[i] - w.x;
        const dy = cy[i] - w.y;
        const d = Math.sqrt(dx * dx + dy * dy);
        const lag = (now - w.t0) * speed - d; // quanto a frente já passou deste ladrilho
        if (lag < 0 || lag >= band) continue;
        const p = lag / band;
        const shape = p < ATTACK ? p / ATTACK : (1 - (p - ATTACK) / (1 - ATTACK)) ** 2;
        keep *= 1 - shape * Math.exp(-d * invFall);
      }
      for (const g of glows) {
        const dx = cx[i] - g.x;
        const dy = cy[i] - g.y;
        const d = Math.sqrt(dx * dx + dy * dy);
        const age = Math.max(0, now - g.t0);
        if (d < 90) keep *= 1 - 0.55 * (1 - age / GLOW_MS) * (1 - d / 90);
      }

      const e = 1 - keep;
      if (e > 0.012) {
        buckets[Math.min(LUT_SIZE - 1, (e * (LUT_SIZE - 1)) | 0)].push(i);
        if (e > 0.5) hot.push(i, e);
      }
    }

    // base pré-renderizada + só os ladrilhos acesos, agrupados por cor
    c.setTransform(1, 0, 0, 1, 0, 0);
    c.globalCompositeOperation = "source-over";
    c.drawImage(base, 0, 0);
    c.setTransform(dpr, 0, 0, dpr, 0, 0);

    const rad = Math.max(2, tile * 0.12);
    for (let b = 1; b < LUT_SIZE; b++) {
      const list = buckets[b];
      if (!list.length) continue;
      c.fillStyle = LUT[b];
      c.beginPath();
      for (const i of list) roundRect(c, tx[i], ty[i], tile, tile, rad);
      c.fill();
    }

    // brilho aditivo: halo nos ladrilhos mais quentes, anel da onda e clarão do clique
    c.globalCompositeOperation = "lighter";
    if (glow > 0) {
      for (let j = 0; j < hot.length; j += 2) {
        const i = hot[j];
        const a = (hot[j + 1] - 0.5) * 2 * glow * 0.32;
        c.fillStyle = `rgba(96,165,250,${a.toFixed(3)})`;
        c.fillRect(tx[i] - 4, ty[i] - 4, tile + 8, tile + 8);
      }
    }
    if (!rm) {
      for (const w of waves) {
        // o timestamp do rAF marca o início do quadro e pode ser anterior ao clique
        const age = Math.max(0, now - w.t0);
        const r = age * speed;
        const ringA = 0.42 * Math.exp(-r * invFall * 1.15) * (glow * 0.6 + 0.4);
        if (ringA > 0.01) {
          c.strokeStyle = `rgba(147,197,253,${ringA.toFixed(3)})`;
          c.lineWidth = 1.5;
          c.beginPath();
          c.arc(w.x, w.y, r, 0, Math.PI * 2);
          c.stroke();
        }
        if (age < FLASH_MS) {
          const p = age / FLASH_MS;
          const R = 30 + p * 190;
          const g = c.createRadialGradient(w.x, w.y, 0, w.x, w.y, R);
          g.addColorStop(0, `rgba(191,219,254,${(0.55 * (1 - p)).toFixed(3)})`);
          g.addColorStop(0.4, `rgba(59,130,246,${(0.35 * (1 - p)).toFixed(3)})`);
          g.addColorStop(1, "rgba(59,130,246,0)");
          c.fillStyle = g;
          c.beginPath();
          c.arc(w.x, w.y, R, 0, Math.PI * 2);
          c.fill();
        }
      }
    }
    c.globalCompositeOperation = "source-over";
  }

  // ── Laço de animação (só roda quando há algo para mostrar) ─────────
  const busy = () => waves.length > 0 || glows.length > 0 || (hover.active && !reduced());
  const wantsFrames = () => busy() || (PARAMS.twinkle && !reduced());
  function schedule() {
    if (!raf && pageVisible && onScreen) raf = requestAnimationFrame(frame);
  }
  function requestRender() {
    dirty = true;
    schedule();
  }
  function frame(now: number) {
    raf = 0;
    if (!pageVisible || !onScreen) return;
    try {
      if (dirty || busy() || now - lastRender >= 1000 / IDLE_FPS - 1) {
        dirty = false;
        lastRender = now;
        render(now);
      }
    } finally {
      // um erro num quadro não pode congelar a animação
      if (wantsFrames()) schedule();
    }
  }

  // ── Interação ──────────────────────────────────────────────────────
  function trigger(x: number, y: number) {
    const now = performance.now();
    if (reduced()) {
      glows.push({ x, y, t0: now });
    } else {
      waves.push({ x, y, t0: now });
      if (waves.length > MAX_WAVES) waves.shift();
    }
    if (!waved) {
      waved = true;
      onFirstWave();
    }
    requestRender();
  }

  const localPoint = (e: PointerEvent): [number, number] => {
    const r = canvas.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  };

  hero.addEventListener("pointerdown", (e) => {
    if (e.pointerType === "mouse" && e.button !== 0) return;
    if ((e.target as Element | null)?.closest(NO_WAVE)) return;
    trigger(...localPoint(e));
  });
  hero.addEventListener("pointermove", (e) => {
    if (e.pointerType !== "mouse") return;
    [hover.x, hover.y] = localPoint(e);
    hover.active = true;
    requestRender();
  });
  hero.addEventListener("pointerleave", () => {
    hover.active = false;
    requestRender();
  });

  document.addEventListener("visibilitychange", () => {
    pageVisible = !document.hidden;
    if (pageVisible) {
      lastTwinkle = 0;
      requestRender();
    }
  });
  new IntersectionObserver(([entry]) => {
    onScreen = entry.isIntersecting;
    if (onScreen) requestRender();
  }).observe(hero);
  let layoutRaf = 0;
  new ResizeObserver(() => {
    if (layoutRaf) return;
    layoutRaf = requestAnimationFrame(() => {
      layoutRaf = 0;
      layout();
    });
  }).observe(hero);
  reduceMq.addEventListener("change", requestRender);

  layout();
}
