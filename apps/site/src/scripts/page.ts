// Comportamento da landing fora do mosaico: botão de download conforme o sistema, dica
// "clique no fundo" e aviso para links que ainda não existem.

import { mountMosaic } from "./mosaic";

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T | null;

type Os = "windows" | "macos" | "linux" | "mobile" | "unknown";

function detectOs(): Os {
  const nav = navigator as Navigator & { userAgentData?: { platform?: string } };
  const ua = nav.userAgent || "";
  const platform = nav.userAgentData?.platform || nav.platform || "";
  if (/android|iphone|ipad|ipod/i.test(ua) || (/mac/i.test(platform) && nav.maxTouchPoints > 1)) {
    return "mobile";
  }
  if (/win/i.test(platform) || /windows/i.test(ua)) return "windows";
  if (/mac/i.test(platform) || /mac os/i.test(ua)) return "macos";
  if (/linux|x11|cros/i.test(`${platform} ${ua}`)) return "linux";
  return "unknown";
}

const NAMES: Partial<Record<Os, string>> = { windows: "Windows", macos: "macOS", linux: "Linux" };

function setupDownload() {
  const label = $("cta-label");
  const also = $("cta-also");
  if (!label || !also) return;
  const os = detectOs();
  const name = NAMES[os];
  if (name) {
    label.textContent = `Baixar para ${name}`;
    const others = Object.entries(NAMES).filter(([k]) => k !== os).map(([, v]) => v);
    also.replaceChildren("Também para ");
    others.forEach((other, i) => {
      if (i > 0) also.append(" e ");
      const a = document.createElement("a");
      a.href = "#";
      a.dataset.soon = "";
      a.textContent = other ?? "";
      also.append(a);
    });
  } else if (os === "mobile") {
    label.textContent = "Ver opções de download";
    also.textContent = "O app é para computador. No celular, use a versão web (plano Pro).";
  }
}

function setupToast() {
  const toast = $("toast");
  if (!toast) return;
  let timer = 0;
  document.addEventListener("click", (e) => {
    if (!(e.target as Element | null)?.closest("[data-soon]")) return;
    e.preventDefault();
    toast.textContent = "Em breve: o CodatOS ainda está em desenvolvimento.";
    toast.classList.add("show");
    clearTimeout(timer);
    timer = window.setTimeout(() => toast.classList.remove("show"), 2200);
  });
}

const hint = $("hint");
let hintDone = false;
setTimeout(() => {
  if (!hintDone) hint?.classList.add("show");
}, 1200);

const hero = $("hero");
const canvas = $<HTMLCanvasElement>("mosaic");
if (hero && canvas) {
  mountMosaic(hero, canvas, () => {
    hintDone = true;
    hint?.classList.add("gone");
  });
}
setupDownload();
setupToast();
