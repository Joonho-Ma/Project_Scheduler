const $ = (id) => document.getElementById(id);

function toDateInputValue(d) {
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function hhmmFromTimeInput(val) {
  // "HH:MM" already
  return val;
}

function buildDueAtRFC3339(dateStr, timeStr) {
  // Use local timezone offset automatically via Date
  // Build a local Date then toISOString-like with offset? easiest: use Date and keep ISO in UTC won't match FixedOffset.
  // Instead: manually create local date-time and append local offset.
  const [y, m, d] = dateStr.split("-").map(Number);
  const [hh, mm] = timeStr.split(":").map(Number);
  const dt = new Date(y, m - 1, d, hh, mm, 0);

  const pad = (n) => String(n).padStart(2, "0");
  const yyyy = dt.getFullYear();
  const MM = pad(dt.getMonth() + 1);
  const DD = pad(dt.getDate());
  const HH = pad(dt.getHours());
  const Min = pad(dt.getMinutes());
  const SS = pad(dt.getSeconds());

  const offMin = -dt.getTimezoneOffset(); // minutes east of UTC
  const sign = offMin >= 0 ? "+" : "-";
  const abs = Math.abs(offMin);
  const offH = pad(Math.floor(abs / 60));
  const offM = pad(abs % 60);

  return `${yyyy}-${MM}-${DD}T${HH}:${Min}:${SS}${sign}${offH}:${offM}`;
}

function setMsg(el, text, kind) {
  el.textContent = text || "";
  el.classList.remove("ok", "err");
  if (kind) el.classList.add(kind);
}

async function apiGet(url) {
  const r = await fetch(url);
  if (!r.ok) throw new Error(await r.text());
  return await r.json();
}

async function apiSend(url, method, bodyObj) {
  const r = await fetch(url, {
    method,
    headers: { "Content-Type": "application/json" },
    body: bodyObj ? JSON.stringify(bodyObj) : undefined,
  });
  if (!r.ok) throw new Error(await r.text());
  // some endpoints return {ok:true}
  const text = await r.text();
  return text ? JSON.parse(text) : null;
}

function fmtRFC3339ToLocal(rfc) {
  try {
    const d = new Date(rfc);
    return d.toLocaleString();
  } catch {
    return rfc;
  }
}

function renderTasks(tasks, nowRFC) {
  $("taskCount").textContent = String(tasks.length);
  $("nowText").textContent = nowRFC ? fmtRFC3339ToLocal(nowRFC) : "-";

  const wrap = $("tasksList");
  wrap.innerHTML = "";

  if (tasks.length === 0) {
    wrap.innerHTML = `<div class="small" style="color:var(--muted)">No tasks for this date.</div>`;
    return;
  }

  for (const t of tasks) {
    const isOverdue = nowRFC && new Date(nowRFC) > new Date(t.due_at);
    const badgeOver = isOverdue ? `<span class="badge overdue">overdue</span>` : "";
    const badgeStatus = `<span class="badge">${t.status}</span>`;

    const div = document.createElement("div");
    div.className = "item";
    div.innerHTML = `
      <div class="left">
        <div class="title">${escapeHtml(t.title)}</div>
        <div class="small">
          due: ${escapeHtml(fmtRFC3339ToLocal(t.due_at))} · duration: ${t.duration_min}m · priority: ${t.priority}
        </div>
        <div class="row" style="gap:8px; align-items:center;">
          ${badgeStatus}
          ${badgeOver}
          ${t.tags && t.tags.length ? `<span class="badge">${escapeHtml(t.tags.join(", "))}</span>` : ""}
        </div>
      </div>
      <div class="actions">
        <button class="iconbtn" data-act="toggle">Toggle</button>
        <button class="iconbtn danger" data-act="del">Delete</button>
      </div>
    `;

    div.querySelector('[data-act="toggle"]').onclick = async () => {
      await apiSend(`/api/tasks/${t.id}/toggle`, "POST");
      await refreshAll();
    };
    div.querySelector('[data-act="del"]').onclick = async () => {
      await apiSend(`/api/tasks/${t.id}`, "DELETE");
      await refreshAll();
    };

    wrap.appendChild(div);
  }
}

function renderPlan(resp) {
  $("planNowText").textContent = resp.now ? fmtRFC3339ToLocal(resp.now) : "-";

  const planWrap = $("planList");
  const unWrap = $("unplannedList");
  planWrap.innerHTML = "";
  unWrap.innerHTML = "";

  if (!resp.plan || resp.plan.length === 0) {
    planWrap.innerHTML = `<div class="small" style="color:var(--muted)">No plan items.</div>`;
  } else {
    for (const p of resp.plan) {
      const div = document.createElement("div");
      div.className = "item";
      const badgeOver = p.is_overdue ? `<span class="badge overdue">overdue</span>` : "";
      div.innerHTML = `
        <div class="left">
          <div class="title">${escapeHtml(p.title)}</div>
          <div class="small">${escapeHtml(fmtRFC3339ToLocal(p.start))} → ${escapeHtml(fmtRFC3339ToLocal(p.end))}</div>
          <div class="row" style="gap:8px; align-items:center;">
            <span class="badge">score ${p.score_breakdown.total}</span>
            <span class="badge">u:${p.score_breakdown.urgency}</span>
            <span class="badge">p:${p.score_breakdown.priority}</span>
            <span class="badge">d:${p.score_breakdown.duration_score}</span>
            ${badgeOver}
          </div>
        </div>
      `;
      planWrap.appendChild(div);
    }
  }

  if (!resp.unplanned || resp.unplanned.length === 0) {
    unWrap.innerHTML = `<div class="small" style="color:var(--muted)">None.</div>`;
  } else {
    for (const u of resp.unplanned) {
      const div = document.createElement("div");
      div.className = "item";
      div.innerHTML = `
        <div class="left">
          <div class="title">${escapeHtml(u.task_id)}</div>
          <div class="small">reason: ${escapeHtml(u.reason)}</div>
        </div>
      `;
      unWrap.appendChild(div);
    }
  }
}

async function loadSettings() {
  const s = await apiGet("/api/settings");
  // s: {day_start, day_end, focus_block_min}
  $("dayStartInput").value = s.day_start;
  $("dayEndInput").value = s.day_end;
  $("focusBlockInput").value = String(s.focus_block_min);
}

async function saveSettings(e) {
  e.preventDefault();
  const msg = $("settingsMsg");
  try {
    setMsg(msg, "Saving...", null);
    const body = {
      day_start: hhmmFromTimeInput($("dayStartInput").value),
      day_end: hhmmFromTimeInput($("dayEndInput").value),
      focus_block_min: Number($("focusBlockInput").value),
    };
    await apiSend("/api/settings", "PUT", body);
    setMsg(msg, "Saved.", "ok");
  } catch (err) {
    setMsg(msg, String(err.message || err), "err");
  }
}

async function refreshTasks() {
  const date = $("dateInput").value;
  const resp = await apiGet(`/api/tasks?date=${encodeURIComponent(date)}`);
  renderTasks(resp.tasks || [], resp.now);
}

async function generatePlan() {
  const date = $("dateInput").value;
  const available = Number($("availInput").value);
  const resp = await apiGet(`/api/plan/today?date=${encodeURIComponent(date)}&available_min=${available}`);
  renderPlan(resp);
}

async function refreshAll() {
  await refreshTasks();
}

function escapeHtml(s) {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

async function createTask(e) {
  e.preventDefault();
  const msg = $("createMsg");
  try {
    setMsg(msg, "Creating...", null);

    const date = $("dateInput").value;
    const dueTime = $("dueTimeInput").value;
    const due_at = buildDueAtRFC3339(date, dueTime);

    const tagsRaw = $("tagsInput").value.trim();
    const tags = tagsRaw ? tagsRaw.split(",").map(x => x.trim()).filter(Boolean) : null;

    const body = {
      title: $("titleInput").value.trim(),
      due_at,
      duration_min: Number($("durationInput").value),
      priority: Number($("priorityInput").value),
      tags,
      notes: $("notesInput").value.trim() || null,
    };

    await apiSend("/api/tasks", "POST", body);
    setMsg(msg, "Added.", "ok");

    // reset some fields
    $("titleInput").value = "";
    $("tagsInput").value = "";
    $("notesInput").value = "";

    await refreshAll();
  } catch (err) {
    setMsg(msg, String(err.message || err), "err");
  }
}

function initDefaults() {
  const today = new Date();
  $("dateInput").value = toDateInputValue(today);

  // default due time next hour
  const hh = String(today.getHours()).padStart(2, "0");
  $("dueTimeInput").value = `${hh}:00`;
}

window.addEventListener("DOMContentLoaded", async () => {
  initDefaults();

  $("refreshBtn").onclick = async () => {
    await refreshAll();
  };

  $("planBtn").onclick = async () => {
    await generatePlan();
  };

  $("createTaskForm").addEventListener("submit", createTask);
  $("settingsForm").addEventListener("submit", saveSettings);

  await loadSettings();
  await refreshAll();
});
