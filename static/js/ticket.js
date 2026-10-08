import { api } from "./api.js";
import { initPage, show, td, userName, STATUS_LABELS, PRIORITY_LABELS } from "./ticket-common.js";

await initPage();
const form = document.getElementById("ticket-form");
const id = new URLSearchParams(location.search).get("id");
const TEXT_FIELDS = ["title", "details", "business_requirements", "technical_requirements", "implementation_notes"];

function fillOptions(select, entries) {
  select.replaceChildren(...entries.map(([value, label]) => {
    const o = document.createElement("option");
    o.value = value;
    o.textContent = label;
    return o;
  }));
}

const users = await api("/api/users/assignable");
const userOptions = [["", "Unassigned"], ...users.map((u) => [String(u.id), u.display_name || u.email])];
fillOptions(form.elements.status, Object.entries(STATUS_LABELS));
fillOptions(form.elements.priority, Object.entries(PRIORITY_LABELS));
fillOptions(form.elements.reviewer_id, userOptions);
fillOptions(form.elements.implementer_id, userOptions);
form.elements.status.value = "backlog";
form.elements.priority.value = "normal";

function ensureUserOption(select, u) {
  if (u && ![...select.options].some((o) => o.value === String(u.id))) {
    const o = document.createElement("option");
    o.value = String(u.id);
    o.textContent = u.name;
    select.append(o);
  }
}

function render(t) {
  document.getElementById("heading").textContent = `${t.key}: ${t.title}`;
  document.title = `${t.key} – Task Manager`;
  for (const f of TEXT_FIELDS) form.elements[f].value = t[f];
  form.elements.status.value = t.status;
  form.elements.priority.value = t.priority;
  ensureUserOption(form.elements.reviewer_id, t.reviewer);
  ensureUserOption(form.elements.implementer_id, t.implementer);
  form.elements.reviewer_id.value = t.reviewer ? String(t.reviewer.id) : "";
  form.elements.implementer_id.value = t.implementer ? String(t.implementer.id) : "";
  const archived = t.status === "archived";
  for (const el of form.elements) {
    if (el.name && el.name !== "status") el.disabled = archived;
  }
  document.getElementById("readonly-note").hidden = !archived;
  document.getElementById("meta").textContent =
    `Created by ${t.created_by.name} on ${t.created_at}. Last updated ${t.updated_at}.`;
  document.getElementById("history-section").hidden = false;
  document.getElementById("history").replaceChildren(...t.history.map((h) => {
    const tr = document.createElement("tr");
    const label = (v) => (v == null ? "" : (h.field === "status" ? STATUS_LABELS : PRIORITY_LABELS)[v] || v);
    tr.append(td(h.changed_at), td(h.field), td(label(h.old_value)), td(label(h.new_value)), td(userName(h.changed_by)));
    return tr;
  }));
}

if (id) {
  try { render(await api(`/api/tickets/${encodeURIComponent(id)}`)); } catch (err) { show(err.message); }
}

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const toId = (v) => (v === "" ? null : Number(v));
  const archived = form.elements.title.disabled;
  const body = archived ? { status: form.elements.status.value } : {
    title: form.elements.title.value,
    details: form.elements.details.value,
    business_requirements: form.elements.business_requirements.value,
    technical_requirements: form.elements.technical_requirements.value,
    implementation_notes: form.elements.implementation_notes.value,
    status: form.elements.status.value,
    priority: form.elements.priority.value,
    reviewer_id: toId(form.elements.reviewer_id.value),
    implementer_id: toId(form.elements.implementer_id.value),
  };
  try {
    if (id) {
      render(await api(`/api/tickets/${encodeURIComponent(id)}`, { method: "PATCH", body }));
      show("");
    } else {
      const t = await api("/api/tickets", { method: "POST", body });
      location.href = `/ticket.html?id=${t.id}`;
    }
  } catch (err) { show(err.message); }
});
