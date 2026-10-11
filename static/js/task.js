import { api } from "./api.js";
import { initPage, show, td, userName, STATUS_LABELS, PRIORITY_LABELS } from "./task-common.js?v=2";

const currentUser = await initPage();
const form = document.getElementById("task-form");
const id = new URLSearchParams(location.search).get("id");
const TEXT_FIELDS = ["title", "details", "business_requirements", "technical_requirements", "implementation_notes"];
const commentsSection = document.getElementById("comments-section");
const commentsList = document.getElementById("comments");
const commentsError = document.getElementById("comments-error");
const commentForm = document.getElementById("comment-form");
let comments = [];
let editingCommentId = null;

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

function showCommentsError(message) {
  commentsError.textContent = message;
  commentsError.hidden = !message;
}

function commentButton(label, handler) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "secondary small";
  button.textContent = label;
  button.addEventListener("click", handler);
  return button;
}

function renderComments() {
  commentsList.replaceChildren(...comments.map((comment) => {
    const item = document.createElement("article");
    item.className = "comment";
    const meta = document.createElement("div");
    meta.className = "comment-meta";
    meta.textContent = `${comment.author_name} · ${comment.created_at}${comment.updated_at !== comment.created_at && !comment.deleted_at ? " (edited)" : ""}`;
    item.append(meta);

    if (comment.deleted_at) {
      const deleted = document.createElement("p");
      deleted.className = "comment-body";
      deleted.textContent = "Comment deleted";
      item.append(deleted);
    } else if (editingCommentId === comment.id) {
      const editForm = document.createElement("form");
      editForm.className = "comment-form";
      const input = document.createElement("textarea");
      input.maxLength = 5000;
      input.required = true;
      input.value = comment.body;
      const actions = document.createElement("div");
      actions.className = "comment-actions";
      const save = document.createElement("button");
      save.type = "submit";
      save.textContent = "Save";
      actions.append(save, commentButton("Cancel", () => {
        editingCommentId = null;
        renderComments();
      }));
      editForm.append(input, actions);
      editForm.addEventListener("submit", async (event) => {
        event.preventDefault();
        save.disabled = true;
        try {
          await api(`/api/tasks/${encodeURIComponent(id)}/comments/${comment.id}`, {
            method: "PATCH",
            body: { body: input.value },
          });
          editingCommentId = null;
          await loadComments();
        } catch (err) {
          showCommentsError(err.message);
          save.disabled = false;
        }
      });
      item.append(editForm);
    } else {
      const body = document.createElement("p");
      body.className = "comment-body";
      body.textContent = comment.body;
      item.append(body);
      if (comment.user_id === currentUser.id || currentUser.is_admin) {
        const actions = document.createElement("div");
        actions.className = "comment-actions";
        actions.append(commentButton("Edit", () => {
          editingCommentId = comment.id;
          renderComments();
        }));
        actions.append(commentButton("Delete", async () => {
          if (!confirm("Delete this comment?")) return;
          try {
            await api(`/api/tasks/${encodeURIComponent(id)}/comments/${comment.id}`, {
              method: "DELETE",
            });
            await loadComments();
          } catch (err) {
            showCommentsError(err.message);
          }
        }));
        item.append(actions);
      }
    }
    return item;
  }));
}

async function loadComments() {
  try {
    comments = await api(`/api/tasks/${encodeURIComponent(id)}/comments`);
    showCommentsError("");
    renderComments();
  } catch (err) {
    showCommentsError(err.message);
  }
}

commentForm.addEventListener("submit", async (event) => {
  event.preventDefault();
  const submit = commentForm.querySelector("button[type=submit]");
  submit.disabled = true;
  try {
    await api(`/api/tasks/${encodeURIComponent(id)}/comments`, {
      method: "POST",
      body: { body: commentForm.elements.body.value },
    });
    commentForm.reset();
    await loadComments();
  } catch (err) {
    showCommentsError(err.message);
  } finally {
    submit.disabled = false;
  }
});

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
    render(await api(`/api/tasks/${encodeURIComponent(id)}/block`, { method: "POST", body: { reason: blockForm.elements.reason.value } }));
    blockForm.reset();
    show("");
  } catch (err) { show(err.message); }
});

unblockBtn.addEventListener("click", async () => {
  try {
    render(await api(`/api/tasks/${encodeURIComponent(id)}/unblock`, { method: "POST" }));
    show("");
  } catch (err) { show(err.message); }
});
const linkForm = document.getElementById("link-form");
const relatedSearchForm = document.getElementById("related-search-form");

function renderRelatedTasks(t) {
  const section = document.getElementById("related-section");
  const list = document.getElementById("related-tasks");
  const noRelated = document.getElementById("no-related-tasks");
  const searchResults = document.getElementById("related-search-results");
  const archived = t.status === "archived";
  section.hidden = false;
  noRelated.hidden = t.related_tasks.length > 0;
  relatedSearchForm.hidden = archived;
  list.replaceChildren(...t.related_tasks.map((related) => {
    const li = document.createElement("li");
    const a = document.createElement("a");
    a.href = `/task.html?id=${encodeURIComponent(related.id)}`;
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
            `/api/tasks/${encodeURIComponent(id)}/related/${related.id}`,
            { method: "DELETE" },
          ));
          show("");
        } catch (err) { show(err.message); }
      });
      li.append(" ", btn);
    }
    return li;
  }));
  const relatedIds = new Set(t.related_tasks.map((related) => related.id));
  searchResults.replaceChildren(...[...searchResults.children].filter((item) => {
    return !relatedIds.has(Number(item.dataset.taskId));
  }));
}

relatedSearchForm.addEventListener("submit", async (e) => {
  e.preventDefault();
  const results = document.getElementById("related-search-results");
  try {
    const query = relatedSearchForm.elements.query.value.trim();
    const matches = await api(`/api/tasks/search?q=${encodeURIComponent(query)}`);
    const current = await api(`/api/tasks/${encodeURIComponent(id)}`);
    const alreadyRelated = new Set(current.related_tasks.map((related) => related.id));
    results.replaceChildren(...matches
      .filter((match) => match.id !== Number(id) && !alreadyRelated.has(match.id))
      .map((match) => {
        const li = document.createElement("li");
        li.dataset.taskId = String(match.id);
        const a = document.createElement("a");
        a.href = `/task.html?id=${encodeURIComponent(match.id)}`;
        a.textContent = `${match.key}: ${match.title}`;
        const btn = document.createElement("button");
        btn.type = "button";
        btn.textContent = "Link";
        btn.addEventListener("click", async () => {
          btn.disabled = true;
          try {
            render(await api(`/api/tasks/${encodeURIComponent(id)}/related`, {
              method: "POST",
              body: { related_task_id: match.id },
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
          render(await api(`/api/tasks/${encodeURIComponent(id)}/links/${l.id}`, { method: "DELETE" }));
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
        const taskPath = `/api/tasks/${encodeURIComponent(id)}/tags`;
        const updated = checkbox.checked
          ? await api(taskPath, { method: "POST", body: { tag_id: tag.id } })
          : await api(`${taskPath}/${tag.id}`, { method: "DELETE" });
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
    render(await api(`/api/tasks/${encodeURIComponent(id)}/links`, { method: "POST", body }));
    linkForm.reset();
    show("");
  } catch (err) { show(err.message); }
});

function render(t) {
  document.getElementById("heading").textContent = `${t.key}: ${t.title}`;
  document.title = `${t.key} – Task Manager`;
  for (const f of TEXT_FIELDS) form.elements[f].value = t[f];
  form.elements.expected_by_date.value = t.expected_by_date || "";
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
  renderRelatedTasks(t);
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
  try {
    render(await api(`/api/tasks/${encodeURIComponent(id)}`));
    commentsSection.hidden = false;
    await loadComments();
  } catch (err) { show(err.message); }
}

form.addEventListener("submit", async (e) => {
  e.preventDefault();
  const toId = (v) => (v === "" ? null : Number(v));
  const archived = form.elements.title.disabled;
  const body = archived ? { status: form.elements.status.value } : {
    title: form.elements.title.value,
    expected_by_date: form.elements.expected_by_date.value || null,
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
      render(await api(`/api/tasks/${encodeURIComponent(id)}`, { method: "PATCH", body }));
      show("");
    } else {
      const t = await api("/api/tasks", { method: "POST", body });
      location.href = `/task.html?id=${t.id}`;
    }
  } catch (err) { show(err.message); }
});
