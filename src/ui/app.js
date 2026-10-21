// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-File-Origin: codegen
//
// Readable source for the paint.type web UI behaviour. Kept in step with the
// inline <script> inside `src/ui/index.html`.
(function () {
  "use strict";
  const canvas = document.getElementById("canvas");
  const ctx = canvas.getContext("2d");
  const status = document.getElementById("status");
  const colourEl = document.getElementById("colour");
  const sizeEl = document.getElementById("size");
  const sizeVal = document.getElementById("size-val");
  const hardnessEl = document.getElementById("hardness");
  const hardnessVal = document.getElementById("hardness-val");
  const dot = document.getElementById("brush-dot");
  let docW = canvas.width;
  let docH = canvas.height;

  function bridgeReady() { return typeof window.__gossamer_invoke === "function"; }

  function invoke(payload) {
    if (!bridgeReady()) return Promise.reject(new Error("Gossamer runtime unavailable"));
    return window.__gossamer_invoke("dispatch", payload);
  }

  function defaultExportPath(ext) {
    const stamp = new Date().toISOString().replace(/[:.]/g, "-");
    return "paint-type-" + stamp + "." + ext;
  }

  function hexToRgba(hex) {
    return {
      r: parseInt(hex.slice(1, 3), 16) / 255,
      g: parseInt(hex.slice(3, 5), 16) / 255,
      b: parseInt(hex.slice(5, 7), 16) / 255,
      a: 1.0
    };
  }

  function currentHardness() {
    return hardnessEl ? Number(hardnessEl.value) / 100 : 0.0;
  }

  function updatePreview() {
    const d = Number(sizeEl.value);
    const h = hardnessEl ? Number(hardnessEl.value) : 0;
    sizeVal.textContent = d + " px";
    if (hardnessVal) hardnessVal.textContent = h + "%";
    dot.style.width = d + "px";
    dot.style.height = d + "px";
    const col = colourEl.value;
    if (h >= 50) {
      dot.style.background = "radial-gradient(circle closest-side, " + col + " 50%, transparent 50%)";
    } else {
      dot.style.background = "radial-gradient(circle, " + col + " 0%, " + col + " 35%, transparent 70%)";
    }
  }

  function blit(dirty) {
    if (!dirty) return;
    if (dirty.x === 0 && dirty.y === 0 && dirty.w > 0 && dirty.h > 0 &&
        (canvas.width !== dirty.w || canvas.height !== dirty.h)) {
      canvas.width = dirty.w;
      canvas.height = dirty.h;
      docW = dirty.w;
      docH = dirty.h;
    }
    const bytes = Uint8ClampedArray.from(atob(dirty.rgba_base64), (c) => c.charCodeAt(0));
    ctx.putImageData(new ImageData(bytes, dirty.w, dirty.h), dirty.x, dirty.y);
  }

  function canvasPos(ev) {
    const rect = canvas.getBoundingClientRect();
    return {
      x: (ev.clientX - rect.left) * (docW / rect.width),
      y: (ev.clientY - rect.top) * (docH / rect.height)
    };
  }

  async function waitForBridge() {
    for (let i = 0; i < 200 && !bridgeReady(); i++) {
      await new Promise((r) => setTimeout(r, 25));
    }
    return bridgeReady();
  }

  async function boot() {
    updatePreview();
    if (!(await waitForBridge())) {
      status.textContent = "IPC bridge unavailable - is this running inside Gossamer?";
      return;
    }
    await invoke({ cmd: "new_doc", w: docW, h: docH });
    const c = hexToRgba(colourEl.value);
    await invoke({ cmd: "set_colour", r: c.r, g: c.g, b: c.b, a: c.a });
    await invoke({ cmd: "set_brush", diameter: Number(sizeEl.value), hardness: currentHardness() });
    status.textContent = "Ready - drag on the canvas to paint.";
  }

  colourEl.addEventListener("input", () => {
    updatePreview();
    const c = hexToRgba(colourEl.value);
    invoke({ cmd: "set_colour", r: c.r, g: c.g, b: c.b, a: c.a }).catch(() => {});
  });
  sizeEl.addEventListener("input", () => {
    updatePreview();
    invoke({ cmd: "set_brush", diameter: Number(sizeEl.value), hardness: currentHardness() }).catch(() => {});
  });
  if (hardnessEl) {
    hardnessEl.addEventListener("input", () => {
      updatePreview();
      invoke({ cmd: "set_brush", diameter: Number(sizeEl.value), hardness: currentHardness() }).catch(() => {});
    });
  }

  let painting = false;
  let pendingMove = null;
  let draining = false;

  async function drainMoves() {
    if (draining) return;
    draining = true;
    while (pendingMove !== null) {
      const p = pendingMove;
      pendingMove = null;
      try {
        const res = await invoke({ cmd: "pointer_move", x: p.x, y: p.y });
        if (res && res.ok === "painted") blit(res.dirty);
      } catch (e) { /* ignore */ }
    }
    draining = false;
  }

  canvas.addEventListener("pointerdown", async (ev) => {
    if (!bridgeReady()) return;
    painting = true;
    canvas.setPointerCapture(ev.pointerId);
    const p = canvasPos(ev);
    try {
      const res = await invoke({ cmd: "pointer_down", x: p.x, y: p.y });
      if (res && res.ok === "painted") blit(res.dirty);
    } catch (e) { /* ignore */ }
  });
  canvas.addEventListener("pointermove", (ev) => {
    if (!painting) return;
    pendingMove = canvasPos(ev);
    drainMoves();
  });
  canvas.addEventListener("pointerup", async () => {
    if (!painting) return;
    painting = false;
    while (draining || pendingMove !== null) {
      await new Promise((r) => setTimeout(r, 4));
      drainMoves();
    }
    try { await invoke({ cmd: "pointer_up" }); } catch (e) { /* ignore */ }
  });

  document.getElementById("save").addEventListener("click", async () => {
    if (!bridgeReady()) { status.textContent = "Cannot save - bridge unavailable."; return; }
    const suggested = defaultExportPath("png");
    let path = suggested;
    try {
      const chosen = await window.__gossamer_invoke("__gossamer_dialog_save", { defaultPath: suggested });
      if (chosen && typeof chosen === "string") path = chosen;
    } catch (e) {
      path = window.prompt("Save file path:", suggested);
    }
    if (!path) return;
    const res = await invoke({ cmd: "save_png", path });
    status.textContent = (res && res.ok === "saved") ? "Saved " + res.path : "Save failed";
  });

  window.addEventListener("load", boot);
})();
