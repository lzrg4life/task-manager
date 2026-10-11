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
const availableTags = await api("/api/tags");
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

const FIELD_LABELS = { link_added: "Link added", link_removed: "Link removed", blocked: "Blocked", unblocked: "Unblocked" };
const blockForm = document.getElementById("block-form");
const unblockBtn = document.getElementById("unblock-btn");

function renderBlocked(t) {
  const archived = t.status === "archived";
  document.getElementById("blocked-section").hidden = false;
  document.getElementById("blocked-note").textContent = t.blocked ? `Blocked: ${t.blocked_reason}` : "Not blocked.";
  blockForm.hidden = t.blocked || archived;
  unblockBtn.hidden = !t.blocked || archived;
}

blockForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  try {
    render(await api(`/api/tickets/${encodeURIComponent(id)}/block`, { method: "POST", body: { reason: blockForm.elements.reason.value } }));
    blockForm.reset();
    show("");
  } catch (err) { show(err.message); }
});

unblockBtn.addEventListener("click", async () => {
  try {
    render(await api(`/api/tickets/${encodeURIComponent(id)}/unblock`, { method: "POST" }));
    show("");
  } catch (err) { show(err.message); }
});
const linkForm = document.getElementById("link-form");
const relatedSearchForm = document.getElementById("related-search-form");

function renderRelatedTickets(t) {
  const section = document.getElementById("related-section");
  const list = document.getElementById("related-tickets");
  const noRelated = document.getElementById("no-related-tickets");
  const searchResults = document.getElementById("related-search-results");
  const archived = t.status === "archived";
  section.hidden = false;
  noRelated.hidden = t.related_tickets.length > 0;
  relatedSearchForm.hidden = archived;
  list.replaceChildren(...t.related_tickets.map((related) => {
    const li = document.createElement("li");
    const a = document.createElement("a");
    a.href = `/ticket.html?id=${encodeURIComponent(related.id)}`;
    a.textContent = `${related.key}: ${related.title}`;
    li.append(a, ` (${STATUS_LABELS[related.status]})`);
    if (!archived && related.status !== "archived") {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "secondary small";
      btn.textContent = "Remove";
      btn.addEventListener("click", async () => {
        try {
          render(await api(
            `/api/tickets/${encodeURIComponent(id)}/related/${related.id}`,
            { method: "DELETE" },
          ));
          show("");
        } catch (err) { show(err.message); }
      });
      li.append(" ", btn);
    }
    return li;
  }));
  const relatedIds = new Set(t.related_tickets.map((related) => related.id));
  searchResults.replaceChildren(...[...searchResults.children].filter((item) => {
    return !relatedIds.has(Number(item.dataset.ticketId));
  }));
}

relatedSearchForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  const results = document.getElementById("related-search-results");
  try {
    const query = relatedSearchForm.elements.query.value.trim();
    const matches = await api(`/api/tickets/search?q=${encodeURIComponent(query)}`);
    const current = await api(`/api/tickets/${encodeURIComponent(id)}`);
    const alreadyRelated = new Set(current.related_tickets.map((related) => related.id));
    results.replaceChildren(...matches
      .filter((match) => match.id !== Number(id) && !alreadyRelated.has(match.id))
      .map((match) => {
        const li = document.createElement("li");
        li.dataset.ticketId = String(match.id);
        const a = document.createElement("a");
        a.href = `/ticket.html?id=${encodeURIComponent(match.id)}`;
        a.textContent = `${match.key}: ${match.title}`;
        const btn = document.createElement("button");
        btn.type = "button";
        btn.textContent = "Link";
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          try {
            render(await api(`/api/tickets/${encodeURIComponent(id)}/related`, {
              method: "POST",
              body: { related_ticket_id: match.id },
            }));
            show("");
          } catch (err) {
            btn.disabled = false;
            show(err.message);
          }
        });
        li.append(a, " ", btn);
        return li;
      }));
    show("");
  } catch (err) { show(err.message); }
});

function renderLinks(t) {
  const archived = t.status === "archived";
  document.getElementById("links-section").hidden = false;
  document.getElementById("no-links").hidden = t.links.length > 0;
  linkForm.hidden = archived;
  document.getElementById("links").replaceChildren(...t.links.map((l) => {
    const li = document.createElement("li");
    const a = document.createElement("a");
    if (/^https?:\/\//i.test(l.url)) a.href = l.url;
    a.target = "_blank";
    a.rel = "noopener noreferrer";
    a.textContent = l.label || l.url;
    a.title = l.url;
    li.append(a);
    if (l.label) li.append(` (${l.url})`);
    if (!archived) {
      const btn = document.createElement("button");
      btn.type = "button";
      btn.className = "secondary small";
      btn.textContent = "Remove";
      btn.addEventListener("click", async () => {
        try {
          render(await api(`/api/tickets/${encodeURIComponent(id)}/links/${l.id}`, { method: "DELETE" }));
          show("");
        } catch (err) { show(err.message); }
      });
      li.append(" ", btn);
    }
    return li;
  }));
}

function renderTags(t) {
  const section = document.getElementById("tags-section");
  const list = document.getElementById("tags");
  const noTags = document.getElementById("no-tags");
  section.hidden = false;
  const assignedArchivedTags = t.tags.filter((tag) => tag.archived);
  const visibleTags = [...availableTags, ...assignedArchivedTags];
  noTags.hidden = visibleTags.length > 0;
  list.replaceChildren(...visibleTags.map((tag) => {
    const label = document.createElement("label");
    label.className = "inline";
    label.title = tag.description;
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = t.tags.some((assigned) => assigned.id === tag.id);
    checkbox.disabled = tag.archived;
    checkbox.addEventListener("change", async () => {
      checkbox.disabled = true;
      try {
        const ticketPath = `/api/tickets/${encodeURIComponent(id)}/tags`;
        const updated = checkbox.checked
          ? await api(ticketPath, { method: "POST", body: { tag_id: tag.id } })
          : await api(`${ticketPath}/${tag.id}`, { method: "DELETE" });
        render(updated);
        show("");
      } catch (err) {
        checkbox.checked = !checkbox.checked;
        checkbox.disabled = false;
        show(err.message);
      }
    });
    label.append(
      checkbox,
      document.createTextNode(` ${tag.name}${tag.archived ? " (archived)" : ""}`),
    );
    return label;
  }));
}

linkForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  const body = { url: linkForm.elements.url.value, label: linkForm.elements.label.value };
  try {
    render(await api(`/api/tickets/${encodeURIComponent(id)}/links`, { method: "POST", body }));
    linkForm.reset();
    show("");
  } catch (err) { show(err.message); }
});

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
  renderLinks(t);
  renderBlocked(t);
  renderRelatedTickets(t);
  renderTags(t);
  document.getElementById("history-section").hidden = false;
  document.getElementById("history").replaceChildren(...t.history.map((h) => {
    const tr = document.createElement("tr");
    const labels = h.field === "status" ? STATUS_LABELS : h.field === "priority" ? PRIORITY_LABELS : {};
    const label = (v) => (v == null ? "" : labels[v] || v);
    tr.append(td(h.changed_at), td(FIELD_LABELS[h.field] || h.field), td(label(h.old_value)), td(label(h.new_value)), td(userName(h.changed_by)));
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
